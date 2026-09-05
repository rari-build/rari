import type { Font, LocalFontOptions } from './types'

export default function localFont(_options: LocalFontOptions): Font {
  throw new Error(
    '`localFont()` from `rari/font/local` must be compiled by the rari Vite plugin. `rari()` is missing from your Vite config, or the call is not a static object literal.',
  )
}
