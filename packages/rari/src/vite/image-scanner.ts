import fs from 'node:fs/promises'
import path from 'node:path'
import { build } from 'rolldown'
import { TSX_EXT_REGEX } from '../shared/regex-constants'

const DEFAULT_IMPORT_REGEX = /import\s+(\w+)\s+from\s+['"]rari\/image['"]/g
const NAMED_IMPORT_REGEX = /import\s+\{[^}]*\b(?:Image\s+as\s+(\w+)|Image)\b[^}]*\}\s+from\s+['"]rari\/image['"]/g
const SRC_REGEX = /src:\s*["']([^"']+)["']/
const ESCAPE_REGEX = /[.*+?^${}()|[\]\\]/g
const SAFE_IDENTIFIER_REGEX = /^[A-Z_$][\w$]*$/i
const WIDTH_REGEX = /width:\s*(\d+)/
const QUALITY_REGEX = /quality:\s*(\d+)/
const PRELOAD_TRUE_REGEX = /preload:\s*(true|!0)/
const PRELOAD_FALSE_REGEX = /preload:\s*(false|!1)/
const SRC_PROP_REGEX = /src=\{?["']([^"']+)["']\}?|src=\{([^}]+)\}/
const WIDTH_PROP_REGEX = /width=\{?(\d+)\}?/
const QUALITY_PROP_REGEX = /quality=\{?(\d+)\}?/
const PRELOAD_PROP_REGEX = /preload(?:=\{?true\}?)?/
const PRELOAD_FALSE_PROP_REGEX = /preload=\{?false\}?/
const DEFAULT_QUALITY = 75

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
    const content = await fs.readFile(fullPath, 'utf8')
    await extractImageUsages(content, fullPath, images)
  }
  catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') {
      console.warn(`[rari] Image scanner: Failed to process ${fullPath}:`, error)
    }
  }
}

class Semaphore {
  private permits: number
  private queue: Array<() => void> = []

  constructor(permits: number) {
    this.permits = permits
  }

  async acquire(): Promise<void> {
    if (this.permits > 0) {
      this.permits--
      return
    }

    return new Promise<void>((resolve) => {
      this.queue.push(resolve)
    })
  }

  release(): void {
    this.permits++
    const resolve = this.queue.shift()
    if (resolve) {
      this.permits--
      resolve()
    }
  }

  async run<T>(fn: () => Promise<T>): Promise<T> {
    await this.acquire()
    try {
      return await fn()
    }
    finally {
      this.release()
    }
  }
}

const fileSemaphore = new Semaphore(50)

async function scanDirectory(dir: string, images: Map<string, ImageUsage>): Promise<void> {
  try {
    await fs.access(dir)
  }
  catch {
    return
  }

  const entries = await fs.readdir(dir, { withFileTypes: true })
  const promises: Promise<void>[] = []

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name)

    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === 'dist')
        continue
      promises.push(scanDirectory(fullPath, images))
    }
    else if (entry.isFile() && TSX_EXT_REGEX.test(entry.name)) {
      promises.push(fileSemaphore.run(() => processFile(fullPath, images)))
    }
  }

  await Promise.all(promises)
}

export async function scanForImageUsage(srcDir: string, additionalDirs: string[] = []): Promise<ImageManifest> {
  const images = new Map<string, ImageUsage>()

  try {
    await fs.access(srcDir)
    await scanDirectory(srcDir, images)
  }
  catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      throw new Error(`Required source directory does not exist: ${srcDir}`)
    }
    throw error
  }

  for (const dir of additionalDirs) {
    try {
      await fs.access(dir)
      await scanDirectory(dir, images)
    }
    catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ENOENT')
        console.warn(`[rari] Image scanner: Failed to scan directory ${dir}:`, error)
    }
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
    external: ['rari/image'],
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

function parseImageImports(code: string): Set<string> {
  const identifiers = new Set<string>()

  for (const match of code.matchAll(DEFAULT_IMPORT_REGEX))
    identifiers.add(match[1])

  for (const match of code.matchAll(NAMED_IMPORT_REGEX)) {
    if (match[1])
      identifiers.add(match[1])
    else
      identifiers.add('Image')
  }

  return identifiers
}

function extractImageIdentifiers(transformedCode: string): Set<string> {
  return parseImageImports(transformedCode)
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
  const key = `${imageUsage.src}:${imageUsage.width ?? 'auto'}:${imageUsage.quality ?? DEFAULT_QUALITY}`

  if (!images.has(key) || imageUsage.preload)
    images.set(key, imageUsage)
}

function extractBalancedBraces(code: string, startIndex: number): string | null {
  let braceCount = 0
  let inString = false
  let stringChar = ''
  let escaped = false
  let templateDepth = 0

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
      if (char === '`')
        templateDepth = 1
      continue
    }

    if (inString && char === stringChar) {
      if (stringChar === '`') {
        templateDepth--
        if (templateDepth === 0) {
          inString = false
          stringChar = ''
        }
      }
      else {
        inString = false
        stringChar = ''
      }
      continue
    }

    if (inString && stringChar === '`' && char === '$' && i + 1 < code.length && code[i + 1] === '{') {
      braceCount++
      i++
      continue
    }

    if (inString && stringChar === '`' && braceCount > 0 && char === '`') {
      templateDepth++
      continue
    }

    if (inString && stringChar === '`' && char === '}' && braceCount > 0) {
      braceCount--
      continue
    }

    if (!inString) {
      if (char === '{') {
        braceCount++
      }
      else if (char === '}') {
        braceCount--
        if (braceCount === 0)
          return code.substring(startIndex + 1, i)
      }
    }
  }

  return null
}

function processImageIdentifiers(transformedCode: string, imageIdentifiers: Set<string>, images: Map<string, ImageUsage>): void {
  for (const identifier of imageIdentifiers) {
    if (!SAFE_IDENTIFIER_REGEX.test(identifier)) {
      console.warn(`[rari] Image scanner: Skipping unsafe identifier: ${identifier}`)
      continue
    }

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
  return parseImageImports(content)
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
    if (!SAFE_IDENTIFIER_REGEX.test(alias)) {
      console.warn(`[rari] Image scanner: Skipping unsafe alias: ${alias}`)
      continue
    }

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

  const key = `${src}:${width ?? 'auto'}:${quality ?? DEFAULT_QUALITY}`

  if (!images.has(key) || preload) {
    images.set(key, {
      src,
      width,
      quality,
      preload,
    })
  }
}
