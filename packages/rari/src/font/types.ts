export type FontDisplay = 'auto' | 'block' | 'swap' | 'fallback' | 'optional'

export type LocalFontSrc =
  | string
  | ReadonlyArray<{
      readonly path: string
      readonly weight?: string | number
      readonly style?: string
    }>

export interface LocalFontOptions {
  readonly src: LocalFontSrc
  readonly display?: FontDisplay
  readonly weight?: string | number
  readonly style?: string
  readonly variable?: string
  readonly preload?: boolean
  readonly fallback?: readonly string[]
  readonly adjustFontFallback?: 'Arial' | 'Times New Roman' | false
  readonly declarations?: ReadonlyArray<{ readonly prop: string; readonly value: string }>
}

export interface GoogleFontOptions {
  readonly weight?: string | number | ReadonlyArray<string | number>
  readonly style?: string | ReadonlyArray<string>
  readonly subsets?: readonly string[]
  readonly display?: FontDisplay
  readonly variable?: string
  readonly preload?: boolean
  readonly fallback?: readonly string[]
  readonly adjustFontFallback?: boolean
  readonly axes?: readonly string[]
}

export interface Font {
  readonly className: string
  readonly style: {
    readonly fontFamily: string
    readonly fontWeight?: number | string
    readonly fontStyle?: string
  }
  readonly variable?: string
}

export interface ResolvedFontFace {
  readonly family: string
  readonly style: string
  readonly weight: string
  readonly display: FontDisplay
  readonly src: ReadonlyArray<{ readonly url: string; readonly format: string }>
  readonly preload: boolean
  readonly variable?: string
  readonly fallback?: readonly string[]
  readonly adjustFontFallback?: 'Arial' | 'Times New Roman' | false
  readonly declarations?: ReadonlyArray<{ readonly prop: string; readonly value: string }>
  readonly unicodeRange?: string
  readonly filePaths: readonly string[]
}
