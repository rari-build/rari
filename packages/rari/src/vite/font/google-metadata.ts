import type { GoogleFontOptions } from '@/font/types'
import { GOOGLE_FONT_SUBSETS } from './google-catalog'

export function warnGoogleFontOptions(family: string, options: GoogleFontOptions): void {
  const preload = options.preload !== false
  const subsets = options.subsets

  if (preload && (subsets == null || subsets.length === 0)) {
    console.warn(
      `[rari/font] ${family}: \`preload\` is true but no \`subsets\` were specified. Specify subsets to avoid downloading unused glyphs, or set \`preload: false\`.`,
    )
  }

  if (subsets == null || subsets.length === 0) return

  const available = GOOGLE_FONT_SUBSETS[family]
  if (available == null) return

  const allowed = new Set(available.map(subset => subset.toLowerCase()))
  const unknown = subsets.filter(subset => !allowed.has(subset.toLowerCase()))
  if (unknown.length === 0) return

  console.warn(
    `[rari/font] ${family}: unknown subset(s) ${unknown.map(subset => JSON.stringify(subset)).join(', ')}. Available: ${available.join(', ')}`,
  )
}
