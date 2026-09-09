import fs from 'node:fs'
import path from 'node:path'
import { ensureCacheDir } from './hash'

const FONT_METADATA_URL = 'https://fonts.google.com/metadata/fonts'
const METADATA_FETCH_TIMEOUT_MS = 30_000
const METADATA_FILE = 'google-fonts-metadata.json'

export interface GoogleFontAxisRange {
  readonly tag: string
  readonly min: number
  readonly max: number
}

export interface GoogleFontFamilyMeta {
  readonly family: string
  readonly subsets: readonly string[]
  readonly axisRanges: readonly GoogleFontAxisRange[]
}

interface GoogleAxisMeta {
  readonly tag: string
  readonly min?: number
  readonly max?: number
}

interface GoogleFamilyMeta {
  readonly family: string
  readonly subsets?: readonly string[]
  readonly axes?: readonly GoogleAxisMeta[]
}

interface GoogleFontsMetadata {
  readonly familyMetadataList: readonly GoogleFamilyMeta[]
}

const indexesByCacheDir = new Map<string, Map<string, GoogleFontFamilyMeta>>()
const pendingByCacheDir = new Map<string, Promise<Map<string, GoogleFontFamilyMeta>>>()

function isRecord(value: unknown): value is Record<string, unknown> {
  return value != null && typeof value === 'object' && !Array.isArray(value)
}

function isGoogleFontsMetadata(value: unknown): value is GoogleFontsMetadata {
  if (!isRecord(value) || !Array.isArray(value.familyMetadataList)) return false
  return value.familyMetadataList.every(
    entry => isRecord(entry) && typeof entry.family === 'string' && entry.family !== '',
  )
}

function parseMetadataPayload(raw: string): unknown {
  const text = raw.replace(/^\)\]\}'\n?/, '')
  return JSON.parse(text) as unknown
}

function uniqueSorted(values: Iterable<string>): string[] {
  return [...new Set(values)].sort((a, b) => a.localeCompare(b))
}

function normalizeFamily(entry: GoogleFamilyMeta): GoogleFontFamilyMeta {
  const subsets = uniqueSorted(
    (entry.subsets ?? []).filter(subset => subset !== 'menu' && subset !== ''),
  )
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

  return {
    family: entry.family,
    subsets,
    axisRanges,
  }
}

function buildIndex(raw: unknown): Map<string, GoogleFontFamilyMeta> {
  if (!isGoogleFontsMetadata(raw)) {
    throw new Error('Unexpected fonts.google.com/metadata/fonts shape')
  }
  const index = new Map<string, GoogleFontFamilyMeta>()
  for (const entry of raw.familyMetadataList) {
    const normalized = normalizeFamily(entry)
    index.set(normalized.family, normalized)
  }
  return index
}

async function loadMetadataIndex(cacheDir: string): Promise<Map<string, GoogleFontFamilyMeta>> {
  const cached = indexesByCacheDir.get(cacheDir)
  if (cached != null) return cached

  const pending = pendingByCacheDir.get(cacheDir)
  if (pending != null) return pending

  const loadPromise = Promise.resolve().then(async () => {
    const filePath = path.join(cacheDir, METADATA_FILE)

    if (fs.existsSync(filePath)) {
      try {
        const index = buildIndex(parseMetadataPayload(fs.readFileSync(filePath, 'utf8')))
        indexesByCacheDir.set(cacheDir, index)
        return index
      } catch {
        try {
          fs.unlinkSync(filePath)
        } catch {
          // Best-effort removal of a corrupt cache entry.
        }
      }
    }

    const response = await fetch(FONT_METADATA_URL, {
      headers: { 'User-Agent': 'rari-google-font-meta' },
      signal: AbortSignal.timeout(METADATA_FETCH_TIMEOUT_MS),
    })
    if (!response.ok) {
      throw new Error(
        `Failed to fetch Google font metadata: ${response.status} ${response.statusText}`,
      )
    }
    const rawText = await response.text()
    const index = buildIndex(parseMetadataPayload(rawText))
    ensureCacheDir(cacheDir)
    fs.writeFileSync(filePath, rawText.replace(/^\)\]\}'\n?/, ''))
    indexesByCacheDir.set(cacheDir, index)
    return index
  })

  pendingByCacheDir.set(cacheDir, loadPromise)
  return loadPromise.finally(() => {
    if (pendingByCacheDir.get(cacheDir) === loadPromise) {
      pendingByCacheDir.delete(cacheDir)
    }
  })
}

export async function loadGoogleFontFamilyMeta(
  cacheDir: string,
  family: string,
): Promise<GoogleFontFamilyMeta | null> {
  try {
    const index = await loadMetadataIndex(cacheDir)
    return index.get(family) ?? null
  } catch (error) {
    console.warn(`[rari/font] Failed to load Google font metadata for ${family}:`, error)
    return null
  }
}

export function resetGoogleFontMetaCacheForTests(): void {
  indexesByCacheDir.clear()
  pendingByCacheDir.clear()
}
