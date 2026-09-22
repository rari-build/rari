import type { UseCacheTransformOptions } from '@rari/use-cache'

type UseCacheTransform = (
  code: string,
  id: string,
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types matches @rari/use-cache transform signature
  options?: UseCacheTransformOptions,
) => string | null

let useCacheTransformPromise: Promise<UseCacheTransform | null> | undefined

export async function getUseCacheTransform(): Promise<UseCacheTransform | null> {
  useCacheTransformPromise ??= (async () => {
    try {
      const module = await import('@rari/use-cache')
      const transform = module.transformUseCacheModule
      return typeof transform === 'function' ? transform : null
    } catch (error) {
      const detail = error instanceof Error && error.message !== '' ? ` ${error.message}` : ''
      throw new Error(
        `\`experimental.useCache\` / \`experimental.useCacheRemote\` requires the optional \`@rari/use-cache\` package. Install it before enabling the option.${detail}`,
      )
    }
  })()
  return useCacheTransformPromise
}
