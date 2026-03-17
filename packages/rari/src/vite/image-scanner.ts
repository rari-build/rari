import fs from 'node:fs'
import path from 'node:path'
import { build } from 'rolldown'
import { TSX_EXT_REGEX } from '../shared/regex-constants'

const DEFAULT_IMPORT_REGEX = /import\s+(\w+)\s+from\s+['"]rari\/image['"]/g
const NAMED_IMPORT_REGEX = /import\s+\{[^}]*\b(?:Image\s+as\s+(\w+)|Image)\b[^}]*\}\s+from\s+['"]rari\/image['"]/g
const SRC_REGEX = /src:\s*["']([^"']+)["']/
const ESCAPE_REGEX = /[.*+?^${}()|[\]\\]/g
const WIDTH_REGEX = /width:\s*(\d+)/
const QUALITY_REGEX = /quality:\s*(\d+)/
const PRELOAD_TRUE_REGEX = /preload:\s*(true|!0)/
const PRELOAD_FALSE_REGEX = /preload:\s*(false|!1)/
const SRC_PROP_REGEX = /src=\{?["']([^"']+)["']\}?|src=\{([^}]+)\}/
const WIDTH_PROP_REGEX = /width=\{?(\d+)\}?/
const QUALITY_PROP_REGEX = /quality=\{?(\d+)\}?/
const PRELOAD_PROP_REGEX = /preload(?:=\{?true\}?)?/
const PRELOAD_FALSE_PROP_REGEX = /preload=\{?false\}?/

export interface ImageUsage {
  src: string
  width?: number
  quality?: number
  preload?: boolean
}

export interface ImageManifest {
  images: ImageUsage[]
}

async function processFile(fullPath: string, images: Map<string, ImageUsage>): Promise<void> {
  try {
    const content = fs.readFileSync(fullPath, 'utf8')

    if (!content.includes('from \'rari/image\'') && !content.includes('from "rari/image"'))
      return

    await extractImageUsages(content, fullPath, images)
  }
  catch (error) {
    console.warn(`Failed to read or process file ${fullPath}:`, error)
  }
}

async function scanDirectory(dir: string, images: Map<string, ImageUsage>): Promise<void> {
  if (!fs.existsSync(dir))
    return

  const entries = fs.readdirSync(dir, { withFileTypes: true })

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name)

    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === 'dist')
        continue
      await scanDirectory(fullPath, images)
    }
    else if (entry.isFile() && TSX_EXT_REGEX.test(entry.name)) {
      await processFile(fullPath, images)
    }
  }
}

export async function scanForImageUsage(srcDir: string, additionalDirs: string[] = []): Promise<ImageManifest> {
  const images = new Map<string, ImageUsage>()

  await scanDirectory(srcDir, images)

  for (const dir of additionalDirs) {
    if (fs.existsSync(dir))
      await scanDirectory(dir, images)
  }

  return {
    images: [...images.values()],
  }
}

function determineLoader(filePath: string): 'tsx' | 'jsx' | 'ts' | 'js' {
  if (filePath.endsWith('.tsx'))
    return 'tsx'
  if (filePath.endsWith('.jsx'))
    return 'jsx'
  if (filePath.endsWith('.ts'))
    return 'ts'

  return 'js'
}

async function transformCode(content: string, filePath: string, loader: 'tsx' | 'jsx' | 'ts' | 'js'): Promise<string> {
  const virtualModuleId = `\0virtual:${filePath}`

  const result = await build({
    input: virtualModuleId,
    platform: 'browser',
    write: false,
    output: {
      format: 'esm',
    },
    moduleTypes: {
      [`.${loader}`]: loader,
    },
    transform: {
      jsx: 'react',
    },
    plugins: [
      {
        name: 'virtual-module',
        resolveId(id) {
          if (id === virtualModuleId)
            return id

          return null
        },
        load(id) {
          if (id === virtualModuleId) {
            return {
              code: content,
              moduleType: loader,
            }
          }

          return null
        },
      },
    ],
  })

  if (!result.output || result.output.length === 0)
    throw new Error('Transform produced no output')

  return result.output[0].code
}

function extractImageIdentifiers(transformedCode: string): Set<string> {
  const imageIdentifiers = new Set<string>()

  for (const match of transformedCode.matchAll(DEFAULT_IMPORT_REGEX))
    imageIdentifiers.add(match[1])

  for (const match of transformedCode.matchAll(NAMED_IMPORT_REGEX)) {
    if (match[1])
      imageIdentifiers.add(match[1])
    else
      imageIdentifiers.add('Image')
  }

  return imageIdentifiers
}

function parseImageProps(propsString: string): ImageUsage | null {
  const srcMatch = propsString.match(SRC_REGEX)
  if (!srcMatch)
    return null

  const src = srcMatch[1]

  if (!src.startsWith('/') && !src.startsWith('http'))
    return null

  const widthMatch = propsString.match(WIDTH_REGEX)
  const width = widthMatch ? Number.parseInt(widthMatch[1], 10) : undefined

  const qualityMatch = propsString.match(QUALITY_REGEX)
  const quality = qualityMatch ? Number.parseInt(qualityMatch[1], 10) : undefined

  const preloadMatch = propsString.match(PRELOAD_TRUE_REGEX)
  const preloadFalseMatch = propsString.match(PRELOAD_FALSE_REGEX)
  const preload = !!preloadMatch && !preloadFalseMatch

  return { src, width, quality, preload }
}

function addImageToMap(imageUsage: ImageUsage, images: Map<string, ImageUsage>): void {
  const key = `${imageUsage.src}:${imageUsage.width || 'auto'}:${imageUsage.quality || 75}`

  if (!images.has(key) || imageUsage.preload)
    images.set(key, imageUsage)
}

function extractBalancedBraces(code: string, startIndex: number): string | null {
  let braceCount = 0
  let inString = false
  let stringChar = ''
  let escaped = false

  for (let i = startIndex; i < code.length; i++) {
    const char = code[i]

    if (escaped) {
      escaped = false
      continue
    }

    if (char === '\\') {
      escaped = true
      continue
    }

    if (!inString && (char === '"' || char === '\'' || char === '`')) {
      inString = true
      stringChar = char
      continue
    }

    if (inString && char === stringChar) {
      inString = false
      stringChar = ''
      continue
    }

    if (!inString) {
      if (char === '{') {
        braceCount++
      }
      else if (char === '}') {
        braceCount--
        if (braceCount === 0) {
          return code.substring(startIndex + 1, i)
        }
      }
    }
  }

  return null
}

function processImageIdentifiers(transformedCode: string, imageIdentifiers: Set<string>, images: Map<string, ImageUsage>): void {
  for (const identifier of imageIdentifiers) {
    const escapedIdentifier = identifier.replace(ESCAPE_REGEX, '\\$&')
    const createElementPattern = new RegExp(`React\\.createElement\\(\\s*${escapedIdentifier}\\s*,\\s*\\{`, 'g')

    for (const match of transformedCode.matchAll(createElementPattern)) {
      const braceStartIndex = match.index! + match[0].length - 1
      const propsString = extractBalancedBraces(transformedCode, braceStartIndex)

      if (propsString) {
        const imageUsage = parseImageProps(propsString)

        if (imageUsage)
          addImageToMap(imageUsage, images)
      }
    }
  }
}

function extractImageAliases(content: string): Set<string> {
  const aliases = new Set<string>()

  for (const match of content.matchAll(DEFAULT_IMPORT_REGEX))
    aliases.add(match[1])

  for (const match of content.matchAll(NAMED_IMPORT_REGEX)) {
    if (match[1])
      aliases.add(match[1])
    else
      aliases.add('Image')
  }

  return aliases
}

async function extractImageUsages(content: string, filePath: string, images: Map<string, ImageUsage>) {
  const aliases = extractImageAliases(content)

  if (aliases.size === 0)
    return

  try {
    const loader = determineLoader(filePath)
    const transformedCode = await transformCode(content, filePath, loader)
    const imageIdentifiers = extractImageIdentifiers(transformedCode)

    if (imageIdentifiers.size === 0)
      return

    processImageIdentifiers(transformedCode, imageIdentifiers, images)
  }
  catch {
    extractImageUsagesWithRegex(content, aliases, images)
  }
}

function extractImageUsagesWithRegex(content: string, aliases: Set<string>, images: Map<string, ImageUsage>) {
  for (const alias of aliases) {
    const escapedAlias = alias.replace(ESCAPE_REGEX, '\\$&')
    const selfClosingRegex = new RegExp(`<${escapedAlias}\\s([^/>]+)\\/>`, 'g')
    const openingRegex = new RegExp(`<${escapedAlias}\\s([^>]+)>`, 'g')

    for (const match of content.matchAll(selfClosingRegex))
      processImageProps(match[1], images)

    for (const match of content.matchAll(openingRegex))
      processImageProps(match[1], images)
  }
}

function processImageProps(propsString: string, images: Map<string, ImageUsage>) {
  const srcMatch = propsString.match(SRC_PROP_REGEX)
  if (!srcMatch)
    return

  const src = srcMatch[1] || srcMatch[2]

  if (!src || src.includes('{') || (!src.startsWith('/') && !src.startsWith('http')))
    return

  const widthMatch = propsString.match(WIDTH_PROP_REGEX)
  const width = widthMatch ? Number.parseInt(widthMatch[1], 10) : undefined

  const qualityMatch = propsString.match(QUALITY_PROP_REGEX)
  const quality = qualityMatch ? Number.parseInt(qualityMatch[1], 10) : undefined

  const preload = PRELOAD_PROP_REGEX.test(propsString) && !PRELOAD_FALSE_PROP_REGEX.test(propsString)

  const key = `${src}:${width || 'auto'}:${quality || 75}`

  if (!images.has(key) || preload) {
    images.set(key, {
      src,
      width,
      quality,
      preload,
    })
  }
}
