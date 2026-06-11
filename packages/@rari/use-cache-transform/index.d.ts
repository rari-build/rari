// Type definitions for @rari/use-cache-transform.

export interface TransformOptions {
  filename: string
  hashSalt?: string
  cacheKinds?: string[]
}

export interface TransformResult {
  code: string
  needsReactCache: boolean
  needsCacheWrapper: boolean
  needsRegisterRef: boolean
}

export interface NativeAddon {
  detectUseCache: (source: string) => boolean
  transformUseCache: (source: string, options: TransformOptions) => TransformResult
}

declare const nativeAddon: NativeAddon

export function detectUseCache(source: string): boolean
export function transformUseCache(
  source: string,
  options: TransformOptions,
): TransformResult

export default nativeAddon
