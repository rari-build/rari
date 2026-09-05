import type { LocalFontOptions, LocalFontSrc, ResolvedFontFace } from '@/font/types'
import fs from 'node:fs'
import path from 'node:path'
import { fontFormatFromPath, normalizeDisplay, normalizeStyle, normalizeWeight } from './css'

function toSrcEntries(
  src: LocalFontSrc,
): Array<{ path: string; weight?: string | number; style?: string }> {
  if (typeof src === 'string') return [{ path: src }]
  return src.map(entry => ({ ...entry }))
}

export function resolveLocalFontFaces(
  options: LocalFontOptions,
  importerDir: string,
  projectRoot: string,
): ResolvedFontFace[] {
  const display = normalizeDisplay(options.display)
  const preload = options.preload !== false
  const entries = toSrcEntries(options.src)

  if (entries.length === 0) {
    throw new Error('rari/font/local: `src` must include at least one font file')
  }

  const resolvedEntries = entries.map(entry => {
    const absolute = path.isAbsolute(entry.path)
      ? entry.path
      : path.resolve(importerDir, entry.path)
    if (!fs.existsSync(absolute)) {
      throw new Error(`rari/font/local: font file not found: ${entry.path} (from ${importerDir})`)
    }

    const relativeToRoot = path.relative(projectRoot, absolute)
    if (relativeToRoot.startsWith('..') || path.isAbsolute(relativeToRoot)) {
      throw new Error(`rari/font/local: font file must be inside the project: ${entry.path}`)
    }

    return {
      absolute,
      weight: normalizeWeight(entry.weight ?? options.weight),
      style: normalizeStyle(entry.style ?? options.style),
      familyBase: path.basename(absolute, path.extname(absolute)),
    }
  })

  const sharedFamily = resolvedEntries[0].familyBase

  return resolvedEntries.map(entry => ({
    family: sharedFamily,
    style: entry.style,
    weight: entry.weight,
    display,
    src: [{ url: '', format: fontFormatFromPath(entry.absolute) }],
    preload,
    variable: options.variable,
    fallback: options.fallback,
    adjustFontFallback: options.adjustFontFallback,
    declarations: options.declarations,
    filePaths: [entry.absolute],
  }))
}
