import type { UseCacheTransformOptions } from '@rari/use-cache'

type UseCacheTransform = (
  code: string,
  id: string,
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types matches @rari/use-cache transform signature
  options?: UseCacheTransformOptions,
) => string | null

let useCacheTransformPromise: Promise<UseCacheTransform> | undefined

export async function getUseCacheTransform(): Promise<UseCacheTransform> {
  useCacheTransformPromise ??= (async () => {
    try {
      const module = await import('@rari/use-cache')
      module.requireNativeAddon()
      const transform = module.transformUseCacheModule
      if (typeof transform !== 'function') {
        throw new TypeError('`@rari/use-cache` did not export transformUseCacheModule')
      }
      return transform
    } catch (error) {
      const detail = error instanceof Error && error.message !== '' ? ` ${error.message}` : ''
      throw new Error(
        `\`experimental.useCache\` / \`experimental.useCacheRemote\` requires a working \`@rari/use-cache\` native addon. Install the optional platform package, or run \`just build-addon-dev\` in the monorepo.${detail}`,
      )
    }
  })()
  return useCacheTransformPromise
}
