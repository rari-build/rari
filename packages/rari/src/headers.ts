import type { CookieStore, ReadonlyHeaders } from './runtime/shared/types'
import { getRariGlobal } from './runtime/shared/rari-global'

export type { CookieOptions, CookieStore, ReadonlyCookie, ReadonlyHeaders } from './runtime/shared/types'

function markUseCacheDynamicContext(): void {
  const rari = getRariGlobal()

  if (rari.markUseCacheDynamic) {
    rari.markUseCacheDynamic()
    return
  }

  rari.useCacheDynamicDepth = (rari.useCacheDynamicDepth ?? 0) + 1
}

export async function cookies(): Promise<CookieStore> {
  markUseCacheDynamicContext()
  const store = getRariGlobal().cookies?.()
  if (!store) {
    throw new Error(
      '[rari] cookies() is only available in server actions and server components.',
    )
  }

  return store
}

export async function headers(): Promise<ReadonlyHeaders> {
  markUseCacheDynamicContext()
  const requestHeaders = getRariGlobal().headers?.()
  if (!requestHeaders) {
    throw new Error(
      '[rari] headers() is only available in server actions and server components.',
    )
  }

  return requestHeaders
}
