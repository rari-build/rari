/// <reference path="../core/types.d.ts" />

(function () {
  if (!g['~rari'])
    g['~rari'] = {}

  const rari = g['~rari']

  function markDynamic() {
    rari.useCacheDynamicDepth = (rari.useCacheDynamicDepth ?? 0) + 1
  }

  rari.invalidateUseCache = async (input: { tag?: string, path?: string }) => {
    const invalidate = g.__rariInvalidateUseCache
    if (typeof invalidate !== 'function')
      return

    if (input.tag)
      await invalidate(input.tag)
    if (input.path)
      await invalidate(input.path)
  }

  rari.markUseCacheDynamic = markDynamic

  const previousCookies = rari.cookies
  rari.cookies = () => {
    if (typeof previousCookies === 'function')
      return previousCookies()
    throw new Error('[rari] cookies() is not available in this runtime context.')
  }
})()
