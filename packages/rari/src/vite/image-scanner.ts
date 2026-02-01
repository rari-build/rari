import fs from 'node:fs'
import path from 'node:path'
import { build } from 'rolldown'

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
      else if (entry.isFile() && /\.(?:tsx?|jsx?)$/.test(entry.name)) {
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

    const defaultImportRegex = /import\s+(\w+)\s+from\s+['"]rari\/image['"]/g
    for (const match of transformedCode.matchAll(defaultImportRegex))
      imageIdentifiers.add(match[1])

    const namedImportRegex = /import\s+\{[^}]*\b(?:Image\s+as\s+(\w+)|Image)\b[^}]*\}\s+from\s+['"]rari\/image['"]/g
    for (const match of transformedCode.matchAll(namedImportRegex)) {
      if (match[1])
        imageIdentifiers.add(match[1])
      else
        imageIdentifiers.add('Image')
    }

    if (imageIdentifiers.size === 0)
      return

    for (const identifier of imageIdentifiers) {
      const escapedIdentifier = identifier.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      const createElementRegex = new RegExp(`React\\.createElement\\(\\s*${escapedIdentifier}\\s*,\\s*\\{([^}]+)\\}`, 'g')

      for (const match of transformedCode.matchAll(createElementRegex)) {
        const propsString = match[1]

        const srcMatch = propsString.match(/src:\s*["']([^"']+)["']/)
        if (!srcMatch)
          continue

        const src = srcMatch[1]

        if (!src.startsWith('/') && !src.startsWith('http'))
          continue

        const widthMatch = propsString.match(/width:\s*(\d+)/)
        const width = widthMatch ? Number.parseInt(widthMatch[1], 10) : undefined

        const qualityMatch = propsString.match(/quality:\s*(\d+)/)
        const quality = qualityMatch ? Number.parseInt(qualityMatch[1], 10) : undefined

        const preloadMatch = propsString.match(/preload:\s*(true|!0)/)
        const preloadFalseMatch = propsString.match(/preload:\s*(false|!1)/)
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
  const selfClosingRegex = /<Image\s([^/>]+)\/>/g
  const openingRegex = /<Image\s([^>]+)>/g

  for (const match of content.matchAll(selfClosingRegex))
    processImageProps(match[1], images)

  for (const match of content.matchAll(openingRegex))
    processImageProps(match[1], images)
}

function processImageProps(propsString: string, images: Map<string, ImageUsage>) {
  const srcMatch = propsString.match(/src=\{?["']([^"']+)["']\}?|src=\{([^}]+)\}/)
  if (!srcMatch)
    return

  const src = srcMatch[1] || srcMatch[2]

  if (!src || src.includes('{') || (!src.startsWith('/') && !src.startsWith('http')))
    return

  const widthMatch = propsString.match(/width=\{?(\d+)\}?/)
  const width = widthMatch ? Number.parseInt(widthMatch[1], 10) : undefined

  const qualityMatch = propsString.match(/quality=\{?(\d+)\}?/)
  const quality = qualityMatch ? Number.parseInt(qualityMatch[1], 10) : undefined

  const preload = /preload(?:=\{?true\}?)?/.test(propsString) && !/preload=\{?false\}?/.test(propsString)

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
