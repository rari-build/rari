type UseCacheTransform = (code: string, id: string, options?: { hashSalt?: string, cacheKinds?: string[] }) => string | null

let useCacheTransform: UseCacheTransform | null | undefined

export async function getUseCacheTransform(): Promise<UseCacheTransform | null> {
  if (useCacheTransform !== undefined)
    return useCacheTransform

  try {
    const module = await import('@rari/use-cache-transform')
    useCacheTransform = module.transformUseCacheModule
    return useCacheTransform
  }
  catch {
    useCacheTransform = null
    return null
  }
}
