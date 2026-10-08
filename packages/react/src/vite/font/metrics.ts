import arial from '@capsizecss/metrics/arial'
import timesNewRoman from '@capsizecss/metrics/timesNewRoman'

export type FallbackFontName = 'Arial' | 'Times New Roman'

export interface FontMetricLike {
  readonly ascent: number
  readonly descent: number
  readonly lineGap: number
  readonly unitsPerEm: number
  readonly xWidthAvg: number
  readonly category?: string
}

export interface FontMetricOverrides {
  readonly fallbackFont: FallbackFontName
  readonly sizeAdjust: string
  readonly ascentOverride: string
  readonly descentOverride: string
  readonly lineGapOverride: string
}

function toPercent(value: number): string {
  return `${(value * 100).toFixed(2)}%`
}

export function computeFallbackOverrides(
  metrics: FontMetricLike,
  fallback: FallbackFontName,
): FontMetricOverrides | null {
  if (
    !Number.isFinite(metrics.unitsPerEm) ||
    metrics.unitsPerEm === 0 ||
    !Number.isFinite(metrics.xWidthAvg)
  ) {
    return null
  }

  const fallbackMetrics = fallback === 'Times New Roman' ? timesNewRoman : arial
  if (
    !Number.isFinite(fallbackMetrics.unitsPerEm) ||
    fallbackMetrics.unitsPerEm === 0 ||
    !Number.isFinite(fallbackMetrics.xWidthAvg)
  ) {
    return null
  }

  const preferredAvg = metrics.xWidthAvg / metrics.unitsPerEm
  const fallbackAvg = fallbackMetrics.xWidthAvg / fallbackMetrics.unitsPerEm
  if (!Number.isFinite(fallbackAvg) || fallbackAvg === 0) return null

  const sizeAdjust = preferredAvg / fallbackAvg
  if (!Number.isFinite(sizeAdjust) || sizeAdjust === 0) return null

  return {
    fallbackFont: fallback,
    sizeAdjust: toPercent(sizeAdjust),
    ascentOverride: toPercent(metrics.ascent / metrics.unitsPerEm / sizeAdjust),
    descentOverride: toPercent(Math.abs(metrics.descent) / metrics.unitsPerEm / sizeAdjust),
    lineGapOverride: toPercent(metrics.lineGap / metrics.unitsPerEm / sizeAdjust),
  }
}

export async function loadMetricsForFamily(familyName: string): Promise<FontMetricLike | null> {
  const slug = familyName.replaceAll(/\s+/g, '').replace(/^(.)/, char => char.toLowerCase())

  try {
    const mod: unknown = await import(`@capsizecss/metrics/${slug}`)
    if (mod == null || typeof mod !== 'object' || !('default' in mod)) return null
    const metrics = mod.default
    if (metrics == null || typeof metrics !== 'object') return null
    if (
      !('ascent' in metrics) ||
      !('descent' in metrics) ||
      !('lineGap' in metrics) ||
      !('unitsPerEm' in metrics) ||
      !('xWidthAvg' in metrics)
    ) {
      return null
    }
    if (
      typeof metrics.ascent !== 'number' ||
      typeof metrics.descent !== 'number' ||
      typeof metrics.lineGap !== 'number' ||
      typeof metrics.unitsPerEm !== 'number' ||
      typeof metrics.xWidthAvg !== 'number'
    ) {
      return null
    }
    return {
      ascent: metrics.ascent,
      descent: metrics.descent,
      lineGap: metrics.lineGap,
      unitsPerEm: metrics.unitsPerEm,
      xWidthAvg: metrics.xWidthAvg,
      category:
        'category' in metrics && typeof metrics.category === 'string'
          ? metrics.category
          : undefined,
    }
  } catch {
    return null
  }
}

export function categoryFallback(category: string | undefined): FallbackFontName {
  return category === 'serif' ? 'Times New Roman' : 'Arial'
}

export type CssFontGeneric = 'serif' | 'sans-serif' | 'monospace'

const CSS_FONT_GENERICS = new Set<string>([
  'serif',
  'sans-serif',
  'monospace',
  'cursive',
  'fantasy',
])

export function cssGenericFromCategory(category: string | undefined): CssFontGeneric {
  if (category === 'serif') return 'serif'
  if (category === 'monospace') return 'monospace'
  return 'sans-serif'
}

export function fallbackIncludesCssGeneric(fallback: readonly string[] | undefined): boolean {
  return (fallback ?? []).some(name => CSS_FONT_GENERICS.has(name))
}
