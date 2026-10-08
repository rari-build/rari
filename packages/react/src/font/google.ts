import type { Font, GoogleFontOptions } from './types'

export type GoogleFontFn = (options?: GoogleFontOptions) => Font

export function googleFont(): Font {
  throw new Error(
    'Google font imports from `rari/font/google` must be compiled by the rari Vite plugin. Add `rari()` to your Vite config, and pass a static options object.',
  )
}

export type { Font, FontDisplay, GoogleFontOptions } from './types'
