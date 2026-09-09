import type { GoogleFontOptions } from '@/font/types'

export function warnGoogleFontOptions(
  family: string,
  options: GoogleFontOptions,
  availableSubsets?: readonly string[] | null,
): void {
  const hasSubsets = options.subsets != null && options.subsets.length > 0
  const preload = hasSubsets ? options.preload !== false : options.preload === true
  const subsets = options.subsets

  if (preload && !hasSubsets) {
    console.warn(
      `[rari/font] ${family}: \`preload\` is true but no \`subsets\` were specified. Specify subsets to avoid downloading unused glyphs, or set \`preload: false\`.`,
    )
  }

  if (subsets == null || subsets.length === 0) return
  if (availableSubsets == null || availableSubsets.length === 0) return

  const allowed = new Set(availableSubsets.map(subset => subset.toLowerCase()))
  const unknown = subsets.filter(subset => !allowed.has(subset.toLowerCase()))
  if (unknown.length === 0) return

  console.warn(
    `[rari/font] ${family}: unknown subset(s) ${unknown.map(subset => JSON.stringify(subset)).join(', ')}. Available: ${availableSubsets.join(', ')}`,
  )
}
