g['~rari'] ??= {}

const rari = g['~rari']

export function markUseCacheDynamic(): void {
  rari.useCacheDynamicDepth = (rari.useCacheDynamicDepth ?? 0) + 1
}

export async function invalidateUseCache(
  input: Readonly<{ tag?: string; path?: string }>,
): Promise<void> {
  const invalidate = g.__rariInvalidateUseCache
  if (typeof invalidate !== 'function') return

  if (input.tag != null && input.tag !== '') await invalidate(input.tag)
  if (input.path != null && input.path !== '') await invalidate(input.path)
}

rari.invalidateUseCache = invalidateUseCache
rari.markUseCacheDynamic = markUseCacheDynamic

const previousCookies = rari.cookies
rari.cookies = () => {
  markUseCacheDynamic()
  if (typeof previousCookies === 'function') return previousCookies()
  throw new Error('[rari] cookies() is not available in this runtime context.')
}
