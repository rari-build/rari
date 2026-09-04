import type { FontMetricOverrides } from './metrics'
import type { FontDisplay, ResolvedFontFace } from '@/font/types'

function cssEscape(value: string): string {
  return value.replaceAll('\\', '\\\\').replaceAll('"', '\\"')
}

function htmlEscapeAttr(value: string): string {
  return value.replaceAll('&', '&amp;').replaceAll('"', '&quot;').replaceAll('<', '&lt;')
}

function formatSource(url: string, format: string): string {
  return `url("${cssEscape(url)}") format("${format}")`
}

export function fontFormatFromPath(filePath: string): string {
  const lower = filePath.toLowerCase()
  if (lower.endsWith('.woff2')) return 'woff2'
  if (lower.endsWith('.woff')) return 'woff'
  if (lower.endsWith('.ttf')) return 'truetype'
  if (lower.endsWith('.otf')) return 'opentype'
  return 'woff2'
}

export function fontMimeType(format: string): string {
  if (format === 'woff') return 'font/woff'
  if (format === 'truetype') return 'font/ttf'
  if (format === 'opentype') return 'font/otf'
  return 'font/woff2'
}

export function serializeFontFaceRule(face: ResolvedFontFace): string {
  const src = face.src.map(entry => formatSource(entry.url, entry.format)).join(', ')
  const declarations = (face.declarations ?? [])
    .map(decl => `  ${decl.prop}: ${decl.value};`)
    .join('\n')

  let css =
    `@font-face {\n` +
    `  font-family: "${cssEscape(face.family)}";\n` +
    `  src: ${src};\n` +
    `  font-display: ${face.display};\n` +
    `  font-weight: ${face.weight};\n` +
    `  font-style: ${face.style};\n`

  if (face.unicodeRange != null && face.unicodeRange !== '') {
    css += `  unicode-range: ${face.unicodeRange};\n`
  }
  if (declarations !== '') css += `${declarations}\n`
  css += `}\n`
  return css
}

export function generateFontFaceCss(
  face: ResolvedFontFace,
  overrides: FontMetricOverrides | null,
  className: string,
  variableClassName: string | null,
): string {
  let css = serializeFontFaceRule(face)

  if (overrides != null) {
    const fallbackFamily = `${face.family} Fallback`
    css +=
      `@font-face {\n` +
      `  font-family: "${cssEscape(fallbackFamily)}";\n` +
      `  src: local("${cssEscape(overrides.fallbackFont)}");\n` +
      `  ascent-override: ${overrides.ascentOverride};\n` +
      `  descent-override: ${overrides.descentOverride};\n` +
      `  line-gap-override: ${overrides.lineGapOverride};\n` +
      `  size-adjust: ${overrides.sizeAdjust};\n` +
      `}\n`
  }

  const stack = buildFontFamilyStack(face.family, face.fallback, overrides != null)
  css += `.${className} {\n  font-family: ${stack};\n}\n`

  if (face.variable != null && variableClassName != null) {
    css += `.${variableClassName} {\n` + `  ${face.variable}: ${stack};\n` + `}\n`
  }

  return css
}

export function buildFontFamilyStack(
  family: string,
  fallback: readonly string[] | undefined,
  includeMetricFallback: boolean,
): string {
  const parts = [`"${cssEscape(family)}"`]
  if (includeMetricFallback) parts.push(`"${cssEscape(`${family} Fallback`)}"`)
  for (const name of fallback ?? []) parts.push(name.includes(' ') ? `"${cssEscape(name)}"` : name)
  parts.push('sans-serif')
  return parts.join(', ')
}

export function preloadLinksForFaces(faces: readonly ResolvedFontFace[]): string {
  const links: string[] = []
  for (const face of faces) {
    if (!face.preload) continue
    for (const entry of face.src) {
      links.push(
        `<link rel="preload" href="${htmlEscapeAttr(entry.url)}" as="font" type="${fontMimeType(entry.format)}" crossorigin />`,
      )
    }
  }
  return links.join('\n')
}

export function normalizeDisplay(display: FontDisplay | undefined): FontDisplay {
  return display ?? 'swap'
}

export function normalizeWeight(weight: string | number | undefined): string {
  if (weight == null) return '400'
  return String(weight)
}

export function normalizeStyle(style: string | undefined): string {
  return style ?? 'normal'
}
