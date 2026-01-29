import fs from 'node:fs'
import path from 'node:path'
import { transformSync } from 'esbuild'

export interface ImageUsage {
  src: string
  width?: number
  quality?: number
  preload?: boolean
}

export interface ImageManifest {
  images: ImageUsage[]
}

export function scanForImageUsage(srcDir: string, additionalDirs: string[] = []): ImageManifest {
  const images = new Map<string, ImageUsage>()

  function scanDirectory(dir: string) {
    if (!fs.existsSync(dir))
      return

    const entries = fs.readdirSync(dir, { withFileTypes: true })

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name)

      if (entry.isDirectory()) {
        if (entry.name === 'node_modules' || entry.name === 'dist')
          continue
        scanDirectory(fullPath)
      }
      else if (entry.isFile() && /\.(?:tsx?|jsx?)$/.test(entry.name)) {
        try {
          const content = fs.readFileSync(fullPath, 'utf8')

          if (!content.includes('from \'rari/image\'') && !content.includes('from "rari/image"'))
            continue

          extractImageUsages(content, fullPath, images)
        }
        catch {}
      }
    }
  }

  scanDirectory(srcDir)

  for (const dir of additionalDirs) {
    if (fs.existsSync(dir))
      scanDirectory(dir)
  }

  return {
    images: [...images.values()],
  }
}

function extractImageUsages(content: string, filePath: string, images: Map<string, ImageUsage>) {
  try {
    const result = transformSync(content, {
      loader: filePath.endsWith('.tsx') || filePath.endsWith('.jsx') ? 'tsx' : 'ts',
      format: 'esm',
      target: 'esnext',
      logLevel: 'silent',
      jsx: 'transform',
      jsxFactory: 'React.createElement',
    })

    const transformedCode = result.code

    const createElementRegex = /React\.createElement\(\s*Image\s*,\s*\{([^}]+)\}/g

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
      const preload = !!preloadMatch

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
  catch {
    extractImageUsagesWithRegex(content, images)
  }
}

function extractImageUsagesWithRegex(content: string, images: Map<string, ImageUsage>) {
  const selfClosingRegex = /<Image\s([^/>]+)\/>/g
  const openingRegex = /<Image\s([^>]+)>/g

  for (const match of content.matchAll(selfClosingRegex)) {
    processImageProps(match[1], images)
  }

  for (const match of content.matchAll(openingRegex)) {
    processImageProps(match[1], images)
  }
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

  const preload = /preload(?:=\{?true\}?)?/.test(propsString)

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
