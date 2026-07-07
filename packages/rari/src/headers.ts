import type { CookieStore, GlobalWithRari, ReadonlyHeaders } from './runtime/shared/types'
import { markUseCacheDynamicContext } from '@rari/use-cache/runtime/cache-dynamic-context'

export type { CookieOptions, CookieStore, ReadonlyCookie, ReadonlyHeaders } from './runtime/shared/types'

export async function cookies(): Promise<CookieStore> {
  markUseCacheDynamicContext()
  const store = (globalThis as unknown as GlobalWithRari)['~rari']?.cookies?.()
  if (!store) {
    throw new Error(
      '[rari] cookies() is only available in server actions and server components.',
    )
  }

  return store
}

export async function headers(): Promise<ReadonlyHeaders> {
  markUseCacheDynamicContext()
  const requestHeaders = (globalThis as unknown as GlobalWithRari)['~rari']?.headers?.()
  if (!requestHeaders) {
    throw new Error(
      '[rari] headers() is only available in server actions and server components.',
    )
  }

  return requestHeaders
}
