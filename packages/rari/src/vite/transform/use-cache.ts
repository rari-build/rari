import type { UseCacheTransformOptions } from '@rari/use-cache'

type UseCacheTransform = (
  code: string,
  id: string,
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- matches @rari/use-cache transform signature
  options?: UseCacheTransformOptions,
) => string | null

let useCacheTransform: UseCacheTransform | null | undefined

export async function getUseCacheTransform(): Promise<UseCacheTransform | null> {
  if (useCacheTransform !== undefined) return useCacheTransform

  try {
    const module = await import('@rari/use-cache')
    const transform = module.transformUseCacheModule
    useCacheTransform = typeof transform === 'function' ? transform : null
    return useCacheTransform
  } catch {
    useCacheTransform = null
    return null
  }
}
