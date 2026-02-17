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
const IMAGE_SELF_CLOSING_REGEX = /<Image\s([^/>]+)\/>/g
const IMAGE_OPENING_REGEX = /<Image\s([^>]+)>/g
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

export async function scanForImageUsage(srcDir: string, additionalDirs: string[] = []): Promise<ImageManifest> {
  const images = new Map<string, ImageUsage>()

  async function scanDirectory(dir: string) {
    if (!fs.existsSync(dir))
      return

    const entries = fs.readdirSync(dir, { withFileTypes: true })

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name)

      if (entry.isDirectory()) {
        if (entry.name === 'node_modules' || entry.name === 'dist')
          continue
        await scanDirectory(fullPath)
      }
      else if (entry.isFile() && TSX_EXT_REGEX.test(entry.name)) {
        try {
          const content = fs.readFileSync(fullPath, 'utf8')

          if (!content.includes('from \'rari/image\'') && !content.includes('from "rari/image"'))
            continue

          await extractImageUsages(content, fullPath, images)
        }
        catch (error) {
          console.warn(`Failed to read or process file ${fullPath}:`, error)
        }
      }
    }
  }

  await scanDirectory(srcDir)

  for (const dir of additionalDirs) {
    if (fs.existsSync(dir))
      await scanDirectory(dir)
  }

  return {
    images: [...images.values()],
  }
}

async function extractImageUsages(content: string, filePath: string, images: Map<string, ImageUsage>) {
  try {
    let loader: 'tsx' | 'jsx' | 'ts' | 'js'
    if (filePath.endsWith('.tsx'))
      loader = 'tsx'
    else if (filePath.endsWith('.jsx'))
      loader = 'jsx'
    else if (filePath.endsWith('.ts'))
      loader = 'ts'

    else
      loader = 'js'

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
            if (id === virtualModuleId) {
              return id
            }

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

    const transformedCode = result.output[0].code

    const imageIdentifiers = new Set<string>()

    for (const match of transformedCode.matchAll(DEFAULT_IMPORT_REGEX))
      imageIdentifiers.add(match[1])

    for (const match of transformedCode.matchAll(NAMED_IMPORT_REGEX)) {
      if (match[1])
        imageIdentifiers.add(match[1])
      else
        imageIdentifiers.add('Image')
    }

    if (imageIdentifiers.size === 0)
      return

    for (const identifier of imageIdentifiers) {
      const escapedIdentifier = identifier.replace(ESCAPE_REGEX, '\\$&')
      const createElementRegex = new RegExp(`React\\.createElement\\(\\s*${escapedIdentifier}\\s*,\\s*\\{([^}]+)\\}`, 'g')

      for (const match of transformedCode.matchAll(createElementRegex)) {
        const propsString = match[1]

        const srcMatch = propsString.match(SRC_REGEX)
        if (!srcMatch)
          continue

        const src = srcMatch[1]

        if (!src.startsWith('/') && !src.startsWith('http'))
          continue

        const widthMatch = propsString.match(WIDTH_REGEX)
        const width = widthMatch ? Number.parseInt(widthMatch[1], 10) : undefined

        const qualityMatch = propsString.match(QUALITY_REGEX)
        const quality = qualityMatch ? Number.parseInt(qualityMatch[1], 10) : undefined

        const preloadMatch = propsString.match(PRELOAD_TRUE_REGEX)
        const preloadFalseMatch = propsString.match(PRELOAD_FALSE_REGEX)
        const preload = !!preloadMatch && !preloadFalseMatch

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
    }
  }
  catch {
    extractImageUsagesWithRegex(content, images)
  }
}

function extractImageUsagesWithRegex(content: string, images: Map<string, ImageUsage>) {
  for (const match of content.matchAll(IMAGE_SELF_CLOSING_REGEX))
    processImageProps(match[1], images)

  for (const match of content.matchAll(IMAGE_OPENING_REGEX))
    processImageProps(match[1], images)
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
