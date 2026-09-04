import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const FONT_METADATA_URL = 'https://fonts.google.com/metadata/fonts'

interface GoogleAxisMeta {
  readonly tag: string
  readonly min?: number
  readonly max?: number
}

interface GoogleFamilyMeta {
  readonly family: string
  readonly subsets?: readonly string[]
  readonly fonts?: Readonly<Record<string, unknown>>
  readonly axes?: readonly GoogleAxisMeta[]
}

interface GoogleFontsMetadata {
  readonly familyMetadataList: readonly GoogleFamilyMeta[]
}

interface NormalizedFamily {
  readonly family: string
  readonly exportName: string
  readonly subsets: readonly string[]
  readonly weights: readonly string[]
  readonly styles: readonly string[]
  readonly axes: readonly string[]
  readonly axisRanges: ReadonlyArray<{
    readonly tag: string
    readonly min: number
    readonly max: number
  }>
  readonly hasVariableWeight: boolean
}

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const googlePath = path.join(root, 'src/font/google.ts')
const catalogPath = path.join(root, 'src/vite/font/google-catalog.ts')

function toExportName(family: string): string {
  return family.replaceAll(' ', '_')
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value != null && typeof value === 'object' && !Array.isArray(value)
}

function isGoogleFamilyMeta(value: unknown): value is GoogleFamilyMeta {
  return isRecord(value) && typeof value.family === 'string' && value.family !== ''
}

function isGoogleFontsMetadata(value: unknown): value is GoogleFontsMetadata {
  if (!isRecord(value) || !Array.isArray(value.familyMetadataList)) return false
  return value.familyMetadataList.every(entry => isGoogleFamilyMeta(entry))
}

function parseMetadataPayload(raw: string): unknown {
  const text = raw.replace(/^\)\]\}'\n?/, '')
  return JSON.parse(text) as unknown
}

function uniqueSorted(values: Iterable<string>): string[] {
  return [...new Set(values)].sort((a, b) => a.localeCompare(b))
}

function normalizeFamily(entry: GoogleFamilyMeta): NormalizedFamily {
  const subsets = uniqueSorted(
    (entry.subsets ?? []).filter(subset => subset !== 'menu' && subset !== ''),
  )

  const weights: string[] = []
  const styles = new Set<string>()
  for (const key of Object.keys(entry.fonts ?? {})) {
    if (key.endsWith('i')) {
      styles.add('italic')
      const weight = key.slice(0, -1)
      if (weight !== '') weights.push(weight)
    } else {
      styles.add('normal')
      weights.push(key)
    }
  }

  const axisRanges = (entry.axes ?? [])
    .filter(
      (axis): axis is GoogleAxisMeta & { tag: string; min: number; max: number } =>
        typeof axis.tag === 'string' &&
        axis.tag !== '' &&
        typeof axis.min === 'number' &&
        typeof axis.max === 'number',
    )
    .map(axis => ({ tag: axis.tag, min: axis.min, max: axis.max }))
    .sort((a, b) => a.tag.localeCompare(b.tag))
  const axes = uniqueSorted(axisRanges.map(axis => axis.tag))
  const hasVariableWeight = axes.includes('wght')
  const styleList = [...styles]
  if (styleList.length === 0) styleList.push('normal')

  return {
    family: entry.family,
    exportName: toExportName(entry.family),
    subsets,
    weights: uniqueSorted(weights),
    styles: uniqueSorted(styleList),
    axes,
    axisRanges,
    hasVariableWeight,
  }
}

function quoteUnion(values: readonly string[]): string {
  if (values.length === 0) return 'never'
  return values.map(value => JSON.stringify(value)).join(' | ')
}

function optionsTypeName(exportName: string): string {
  return `${exportName}Options`
}

function emitOptionsType(family: NormalizedFamily): string[] {
  const lines: string[] = []
  const name = optionsTypeName(family.exportName)

  const weightLiterals = [...family.weights]
  if (family.hasVariableWeight) weightLiterals.push('variable')
  const weightUnion = quoteUnion(weightLiterals)
  const styleUnion = quoteUnion(family.styles)
  const subsetUnion = quoteUnion(family.subsets)
  const axisUnion = quoteUnion(family.axes)

  lines.push(`interface ${name} {`)
  if (weightLiterals.length > 0) {
    const weightType = family.hasVariableWeight
      ? `${weightUnion} | number | \`\${number} \${number}\` | ReadonlyArray<${weightUnion} | number>`
      : `${weightUnion} | number | ReadonlyArray<${weightUnion} | number>`
    lines.push(`  weight?: ${weightType}`)
  } else {
    lines.push(`  weight?: GoogleFontOptions['weight']`)
  }
  lines.push(`  style?: ${styleUnion} | ReadonlyArray<${styleUnion}>`)
  if (family.subsets.length > 0) {
    lines.push(`  subsets?: ReadonlyArray<${subsetUnion}>`)
  } else {
    lines.push(`  subsets?: GoogleFontOptions['subsets']`)
  }
  lines.push(`  display?: FontDisplay`)
  lines.push(`  variable?: string`)
  lines.push(`  preload?: boolean`)
  lines.push(`  fallback?: readonly string[]`)
  lines.push(`  adjustFontFallback?: boolean`)
  if (family.axes.length > 0) {
    lines.push(`  axes?: ReadonlyArray<${axisUnion}>`)
  } else {
    lines.push(`  axes?: GoogleFontOptions['axes']`)
  }
  lines.push(`}`)
  return lines
}

const response = await fetch(FONT_METADATA_URL, {
  headers: { 'User-Agent': 'rari-update-google-fonts' },
})
if (!response.ok) {
  throw new Error(`Failed to fetch font metadata: ${response.status} ${response.statusText}`)
}

const raw = parseMetadataPayload(await response.text())
if (!isGoogleFontsMetadata(raw)) {
  throw new Error('Unexpected fonts.google.com/metadata/fonts shape')
}

const byExport = new Map<string, NormalizedFamily>()
for (const entry of raw.familyMetadataList) {
  const normalized = normalizeFamily(entry)
  byExport.set(normalized.exportName, normalized)
}

const families = [...byExport.values()].sort((a, b) => a.family.localeCompare(b.family))

const googleOut: string[] = []
googleOut.push('/* Auto-generated by scripts/update-google-fonts.ts */')
googleOut.push("import type { Font, FontDisplay, GoogleFontOptions } from './types'")
googleOut.push('')
googleOut.push(
  'export type GoogleFontFn<Options = GoogleFontOptions> = (options?: Options) => Font',
)
googleOut.push('')
googleOut.push('function createGoogleFontStub(family: string): GoogleFontFn {')
googleOut.push('  return function googleFont(_options: GoogleFontOptions = {}): Font {')
googleOut.push('    throw new Error(')
googleOut.push(
  '      `\\`$' +
    '{family}()\\` from \\`rari/font/google\\` must be compiled by the rari Vite plugin. Add \\`rari()\\` to your Vite config, and pass a static options object.`,',
)
googleOut.push('    )')
googleOut.push('  }')
googleOut.push('}')
googleOut.push('')

for (const family of families) {
  googleOut.push(...emitOptionsType(family))
  googleOut.push(
    `export const ${family.exportName}: GoogleFontFn<${optionsTypeName(family.exportName)}> = /* #__PURE__ */ createGoogleFontStub(${JSON.stringify(family.exportName)})`,
  )
  googleOut.push('')
}

googleOut.push('export type { Font, FontDisplay, GoogleFontOptions }')
googleOut.push('')

fs.writeFileSync(googlePath, googleOut.join('\n'))

const catalogOut: string[] = []
catalogOut.push('/* Auto-generated by scripts/update-google-fonts.ts */')
catalogOut.push('')
catalogOut.push('export interface GoogleFontAxisRange {')
catalogOut.push('  readonly tag: string')
catalogOut.push('  readonly min: number')
catalogOut.push('  readonly max: number')
catalogOut.push('}')
catalogOut.push('')
catalogOut.push('/** Family display name -> allowed subsets (excludes `menu`). */')
catalogOut.push(
  'export const GOOGLE_FONT_SUBSETS: Readonly<Partial<Record<string, readonly string[]>>> = {',
)
for (const family of families) {
  catalogOut.push(`  ${JSON.stringify(family.family)}: ${JSON.stringify(family.subsets)},`)
}
catalogOut.push('}')
catalogOut.push('')
catalogOut.push('/** Family display name -> variable axis ranges from Google metadata. */')
catalogOut.push(
  'export const GOOGLE_FONT_AXES: Readonly<Partial<Record<string, readonly GoogleFontAxisRange[]>>> = {',
)
for (const family of families) {
  if (family.axisRanges.length === 0) continue
  catalogOut.push(`  ${JSON.stringify(family.family)}: ${JSON.stringify(family.axisRanges)},`)
}
catalogOut.push('}')
catalogOut.push('')

fs.writeFileSync(catalogPath, catalogOut.join('\n'))

const formatTargets = [path.relative(root, googlePath), path.relative(root, catalogPath)]
const format = spawnSync('pnpm', ['exec', 'vp', 'fmt', ...formatTargets], {
  cwd: root,
  encoding: 'utf8',
})
if (format.status !== 0) {
  throw new Error(
    `Failed to format generated Google font files:\n${format.stderr || format.stdout || `exit ${format.status}`}`,
  )
}

console.log(`Updated ${families.length} Google fonts -> ${formatTargets.join(' + ')}`)
