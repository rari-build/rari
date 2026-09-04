import type { GoogleFontOptions, ResolvedFontFace } from '@/font/types'
import fs from 'node:fs'
import path from 'node:path'
import { fontFormatFromPath, normalizeDisplay, normalizeStyle, normalizeWeight } from './css'
import { GOOGLE_FONT_AXES } from './google-catalog'
import { warnGoogleFontOptions } from './google-metadata'
import { contentHash, ensureCacheDir } from './hash'

const GOOGLE_CSS_USER_AGENT =
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.0.0 Safari/537.36'

const FONT_FETCH_TIMEOUT_MS = 10_000

export const FONT_PRELOAD_PREFIX = 'preload:'

function toGoogleFamilyParam(family: string): string {
  return family.trim().replaceAll(' ', '+')
}

function weightList(weight: GoogleFontOptions['weight']): string[] {
  if (weight == null) return ['400']
  if (Array.isArray(weight)) {
    const values = weight.map(String)
    return values.length > 0 ? values : ['400']
  }
  return [String(weight)]
}

const VARIABLE_WEIGHT_RANGE_RE = /^(\d+(?:\.\d+)?)\s+(\d+(?:\.\d+)?)$/

function isWeightRange(value: string): boolean {
  return VARIABLE_WEIGHT_RANGE_RE.test(value.trim())
}

function isVariableWeight(weight: GoogleFontOptions['weight']): boolean {
  if (weight === 'variable') return true
  if (typeof weight === 'string' && isWeightRange(weight)) return true
  if (Array.isArray(weight)) {
    return weight.some(
      value => value === 'variable' || (typeof value === 'string' && isWeightRange(value)),
    )
  }
  return false
}

export function variableWeightRange(weight: GoogleFontOptions['weight']): string {
  if (typeof weight === 'string') {
    const match = VARIABLE_WEIGHT_RANGE_RE.exec(weight.trim())
    if (match != null) return `${match[1]}..${match[2]}`
  }
  if (Array.isArray(weight)) {
    for (const value of weight) {
      if (typeof value !== 'string') continue
      const match = VARIABLE_WEIGHT_RANGE_RE.exec(value.trim())
      if (match != null) return `${match[1]}..${match[2]}`
    }
  }
  return '100..900'
}

function catalogAxisRange(family: string, tag: string): string | null {
  const axes = GOOGLE_FONT_AXES[family]
  if (axes == null) return null
  const axis = axes.find(entry => entry.tag === tag)
  if (axis == null) return null
  return `${axis.min}..${axis.max}`
}

function wghtAxisRange(family: string, weight: GoogleFontOptions['weight']): string {
  if (isVariableWeight(weight)) {
    if (typeof weight === 'string' && isWeightRange(weight)) return variableWeightRange(weight)
    if (Array.isArray(weight)) {
      for (const value of weight) {
        if (typeof value === 'string' && isWeightRange(value)) return variableWeightRange(value)
      }
    }
    return catalogAxisRange(family, 'wght') ?? variableWeightRange(weight)
  }
  if (weight != null) {
    const weights = weightList(weight)
    if (weights.length === 1) return `${weights[0]}..${weights[0]}`
  }
  return catalogAxisRange(family, 'wght') ?? '100..900'
}

function styleValues(style: GoogleFontOptions['style']): string[] {
  if (style == null) return ['normal']
  if (typeof style === 'string') return [style]
  const values = style.filter(value => value !== '')
  return values.length > 0 ? [...values] : ['normal']
}

function axisNames(options: GoogleFontOptions): string[] {
  const axes = (options.axes ?? []).map(axis => axis.trim()).filter(Boolean)
  const unique: string[] = []
  for (const axis of axes) {
    if (!unique.includes(axis)) unique.push(axis)
  }
  return unique
}

function staticWeightsForAxes(weight: GoogleFontOptions['weight']): string[] | null {
  if (weight == null || isVariableWeight(weight)) return null
  return weightList(weight)
}

export function buildGoogleCssUrl(family: string, options: GoogleFontOptions): string {
  const familyParam = toGoogleFamilyParam(family)
  const display = normalizeDisplay(options.display)
  const styles = styleValues(options.style)
  const hasItalic = styles.includes('italic')
  const hasNormal = styles.includes('normal') || !hasItalic
  const extraAxes = axisNames(options)
  const variable = isVariableWeight(options.weight) || extraAxes.length > 0

  let axis: string
  if (variable) {
    const named = ['wght', ...extraAxes.filter(name => name !== 'wght' && name !== 'ital')].sort(
      (a, b) => a.localeCompare(b),
    )
    const staticWeights = staticWeightsForAxes(options.weight)
    const axisValue = (name: string, wghtValue: string) =>
      name === 'wght' ? wghtValue : (catalogAxisRange(family, name) ?? '1..1000')

    if (staticWeights != null) {
      const tuples = staticWeights.map(weight =>
        named.map(name => axisValue(name, weight)).join(','),
      )
      if (hasItalic && hasNormal) {
        axis = `ital,${named.join(',')}@${[
          ...tuples.map(tuple => `0,${tuple}`),
          ...tuples.map(tuple => `1,${tuple}`),
        ].join(';')}`
      } else if (hasItalic) {
        axis = `ital,${named.join(',')}@${tuples.map(tuple => `1,${tuple}`).join(';')}`
      } else if (named.length === 1 && named[0] === 'wght') {
        axis = `wght@${staticWeights.join(';')}`
      } else {
        axis = `${named.join(',')}@${tuples.join(';')}`
      }
    } else {
      const wghtRange = wghtAxisRange(family, options.weight)
      const values = named.map(name => axisValue(name, wghtRange))
      if (hasItalic && hasNormal) {
        axis = `ital,${named.join(',')}@0,${values.join(',')};1,${values.join(',')}`
      } else if (hasItalic) {
        axis = `ital,${named.join(',')}@1,${values.join(',')}`
      } else if (named.length === 1 && named[0] === 'wght') {
        axis = `wght@${values[0]}`
      } else {
        axis = `${named.join(',')}@${values.join(',')}`
      }
    }
  } else {
    const weights = weightList(options.weight)
    if (hasItalic && hasNormal) {
      const pairs = [
        ...weights.map(weight => `0,${weight}`),
        ...weights.map(weight => `1,${weight}`),
      ]
      axis = `ital,wght@${pairs.join(';')}`
    } else if (hasItalic) {
      axis = `ital,wght@${weights.map(weight => `1,${weight}`).join(';')}`
    } else {
      axis = `wght@${weights.join(';')}`
    }
  }

  return `https://fonts.googleapis.com/css2?family=${familyParam}:${axis}&display=${display}`
}

export interface ParsedGoogleFontFace {
  readonly family: string
  readonly style: string
  readonly weight: string
  readonly url: string
  readonly subset: string | null
  readonly unicodeRange: string | null
}

export function parseCssFontFaces(css: string): ParsedGoogleFontFace[] {
  const faces: ParsedGoogleFontFace[] = []
  const parts = css.split('@font-face')
  for (let i = 1; i < parts.length; i += 1) {
    const before = parts[i - 1] ?? ''
    const after = parts[i] ?? ''
    const braceStart = after.indexOf('{')
    const braceEnd = after.indexOf('}', braceStart)
    if (braceStart === -1 || braceEnd === -1) continue
    const block = after.slice(braceStart + 1, braceEnd)

    const commentMatch = /\/\*\s*([\w-]+)\s*\*\/\s*$/.exec(before)
    const subset = commentMatch?.[1] ?? null

    const family =
      /font-family:\s*'([^']+)'/.exec(block)?.[1] ?? /font-family:\s*"([^"]+)"/.exec(block)?.[1]
    const style = /font-style:\s*([^;\s]+)\s*;/.exec(block)?.[1] ?? 'normal'
    const weight = /font-weight:([^;]+);/.exec(block)?.[1]?.trim() ?? '400'
    const url = /src:\s*url\(([^)]+)\)/.exec(block)?.[1]?.replaceAll(/['"]/g, '')
    const unicodeRange = /unicode-range:([^;]+);/.exec(block)?.[1]?.trim() ?? null
    if (family != null && url != null && url !== '') {
      faces.push({ family, style, weight, url, subset, unicodeRange })
    }
  }
  return faces
}

export function filterFacesBySubsets(
  faces: readonly ParsedGoogleFontFace[],
  subsets: readonly string[] | undefined,
): ParsedGoogleFontFace[] {
  if (subsets == null || subsets.length === 0) return [...faces]
  const wanted = new Set(subsets.map(subset => subset.toLowerCase()))
  return faces.filter(face => face.subset != null && wanted.has(face.subset.toLowerCase()))
}

async function downloadToCache(url: string, cachePath: string): Promise<Buffer> {
  if (fs.existsSync(cachePath)) return fs.readFileSync(cachePath)
  const response = await fetch(url, { signal: AbortSignal.timeout(FONT_FETCH_TIMEOUT_MS) })
  if (!response.ok) {
    throw new Error(`Failed to download font ${url}: ${response.status} ${response.statusText}`)
  }
  const buffer = Buffer.from(await response.arrayBuffer())
  ensureCacheDir(path.dirname(cachePath))
  fs.writeFileSync(cachePath, buffer)
  return buffer
}

export async function resolveGoogleFontFaces(
  family: string,
  options: GoogleFontOptions,
  cacheDir: string,
): Promise<ResolvedFontFace[]> {
  warnGoogleFontOptions(family, options)

  const cssUrl = buildGoogleCssUrl(family, options)
  const cssCachePath = path.join(cacheDir, `${contentHash(cssUrl)}.css`)
  let css: string
  if (fs.existsSync(cssCachePath)) {
    css = fs.readFileSync(cssCachePath, 'utf8')
  } else {
    const response = await fetch(cssUrl, {
      headers: { 'User-Agent': GOOGLE_CSS_USER_AGENT },
      signal: AbortSignal.timeout(FONT_FETCH_TIMEOUT_MS),
    })
    if (!response.ok) {
      throw new Error(
        `Failed to fetch Google Font CSS for ${family}: ${response.status} ${response.statusText}`,
      )
    }
    css = await response.text()
    ensureCacheDir(cacheDir)
    fs.writeFileSync(cssCachePath, css)
  }

  const parsed = filterFacesBySubsets(parseCssFontFaces(css), options.subsets)
  if (parsed.length === 0) {
    throw new Error(`No @font-face rules returned for Google font "${family}"`)
  }

  const display = normalizeDisplay(options.display)
  const preload = options.preload !== false
  const faces: ResolvedFontFace[] = []

  for (const face of parsed) {
    const fileHash = contentHash(face.url)
    const ext = path.extname(new URL(face.url).pathname) || '.woff2'
    const fileName = `${family.replaceAll(/\s+/g, '')}-${face.weight}-${face.style}-${fileHash}${ext}`
    const filePath = path.join(cacheDir, fileName)
    await downloadToCache(face.url, filePath)

    faces.push({
      family: face.family,
      style: normalizeStyle(face.style),
      weight: normalizeWeight(face.weight),
      display,
      src: [{ url: '', format: fontFormatFromPath(filePath) }],
      preload,
      variable: options.variable,
      fallback: options.fallback,
      adjustFontFallback: options.adjustFontFallback === false ? false : undefined,
      unicodeRange: face.unicodeRange ?? undefined,
      filePaths: [filePath],
    })
  }

  return faces
}

export function googleExportNameToFamily(exportName: string): string {
  return exportName.replaceAll('_', ' ')
}

export function fontPreloadMarker(url: string): string {
  return `${FONT_PRELOAD_PREFIX}${url}`
}

export function isFontPreloadMarker(value: string): boolean {
  return value.startsWith(FONT_PRELOAD_PREFIX)
}

export function fontUrlFromPreloadMarker(value: string): string {
  return value.slice(FONT_PRELOAD_PREFIX.length)
}
