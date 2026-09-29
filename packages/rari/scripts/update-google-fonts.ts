import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const FONT_METADATA_URL = 'https://fonts.google.com/metadata/fonts'
const METADATA_FETCH_TIMEOUT_MS = 30_000

interface GoogleFamilyMeta {
  readonly family: string
}

interface GoogleFontsMetadata {
  readonly familyMetadataList: readonly GoogleFamilyMeta[]
}

interface NormalizedFamily {
  readonly family: string
  readonly exportName: string
}

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const familiesPath = path.join(root, 'src/font/google-families.json')
const distFontDts = path.join(root, 'dist/font.d.mts')
const googleTypesOut = path.join(root, 'dist/font/google-catalog.d.mts')

const TS_RESERVED = new Set([
  'break',
  'case',
  'catch',
  'class',
  'const',
  'continue',
  'debugger',
  'default',
  'delete',
  'do',
  'else',
  'enum',
  'export',
  'extends',
  'false',
  'finally',
  'for',
  'function',
  'if',
  'import',
  'in',
  'instanceof',
  'new',
  'null',
  'return',
  'super',
  'switch',
  'this',
  'throw',
  'true',
  'try',
  'typeof',
  'var',
  'void',
  'while',
  'with',
  'yield',
  'let',
  'static',
  'implements',
  'interface',
  'package',
  'private',
  'protected',
  'public',
  'await',
  'async',
  'of',
  'as',
  'from',
  'type',
])

function toExportName(family: string): string {
  const exportName = family.replaceAll(' ', '_')
  if (!/^[a-z_$][\w$]*$/i.test(exportName) || TS_RESERVED.has(exportName)) {
    throw new Error(
      `Google font family ${JSON.stringify(family)} normalizes to invalid TypeScript export name ${JSON.stringify(exportName)}`,
    )
  }
  return exportName
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

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every(entry => typeof entry === 'string' && entry !== '')
}

function parseMetadataPayload(raw: string): unknown {
  const text = raw.replace(/^\)\]\}'\n?/, '')
  return JSON.parse(text) as unknown
}

function normalizeFamily(entry: GoogleFamilyMeta): NormalizedFamily {
  return {
    family: entry.family,
    exportName: toExportName(entry.family),
  }
}

function readFamilies(): string[] {
  const raw: unknown = JSON.parse(fs.readFileSync(familiesPath, 'utf8'))
  if (!isStringArray(raw)) {
    throw new Error(`Expected string[] in ${familiesPath}`)
  }
  const seen = new Set<string>()
  for (const name of raw) {
    if (toExportName(name) !== name || seen.has(name)) {
      throw new Error(`Invalid or duplicate export name ${JSON.stringify(name)} in ${familiesPath}`)
    }
    seen.add(name)
  }
  return raw
}

function writeGoogleFontTypes(families: readonly string[]): void {
  if (!fs.existsSync(distFontDts)) {
    throw new Error(
      `Missing ${path.relative(root, distFontDts)}. Run \`vp pack\` before emitting google-catalog.d.mts.`,
    )
  }

  const lines: string[] = [
    'import type { Font, FontDisplay, GoogleFontOptions } from "../font.d.mts"',
    '',
    'export type GoogleFontFn = (options?: GoogleFontOptions) => Font',
    '',
  ]

  for (const name of families) {
    lines.push(`export declare const ${name}: GoogleFontFn`)
  }

  lines.push('')
  lines.push('export type { Font, FontDisplay, GoogleFontOptions }')
  lines.push('')

  fs.mkdirSync(path.dirname(googleTypesOut), { recursive: true })
  fs.writeFileSync(googleTypesOut, lines.join('\n'))
  console.log(
    `Wrote ${families.length} Google font type exports -> ${path.relative(root, googleTypesOut)}`,
  )
}

async function fetchFamilies(): Promise<string[]> {
  let response: Response
  try {
    response = await fetch(FONT_METADATA_URL, {
      headers: { 'User-Agent': 'rari-update-google-fonts' },
      signal: AbortSignal.timeout(METADATA_FETCH_TIMEOUT_MS),
    })
  } catch (error) {
    const timedOut =
      error instanceof Error && (error.name === 'TimeoutError' || error.name === 'AbortError')
    if (timedOut) {
      throw new Error(
        `Timed out fetching Google font metadata from ${FONT_METADATA_URL} after ${METADATA_FETCH_TIMEOUT_MS}ms`,
      )
    }
    throw error
  }
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
    const existing = byExport.get(normalized.exportName)
    if (existing != null && existing.family !== normalized.family) {
      throw new Error(
        `Duplicate Google font export name ${JSON.stringify(normalized.exportName)} for families ${JSON.stringify(existing.family)} and ${JSON.stringify(normalized.family)}`,
      )
    }
    byExport.set(normalized.exportName, normalized)
  }

  return [...byExport.values()]
    .sort((a, b) => a.family.localeCompare(b.family))
    .map(family => family.exportName)
}

const typesOnly = process.argv.includes('--types-only')

if (typesOnly) {
  writeGoogleFontTypes(readFamilies())
} else {
  const families = await fetchFamilies()
  fs.writeFileSync(familiesPath, `${JSON.stringify(families)}\n`)
  console.log(
    `Updated ${families.length} Google font families -> ${path.relative(root, familiesPath)}`,
  )

  if (fs.existsSync(distFontDts)) {
    writeGoogleFontTypes(families)
  } else {
    console.log(
      'Skipped dist type emit (run `pnpm --filter rari build` to generate google-catalog.d.mts)',
    )
  }
}
