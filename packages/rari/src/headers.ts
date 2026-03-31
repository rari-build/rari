import type { CookieStore, GlobalWithRari } from './runtime/shared/types'

export type { CookieOptions, CookieStore, ReadonlyCookie } from './runtime/shared/types'

export async function cookies(): Promise<CookieStore> {
  const store = (globalThis as unknown as GlobalWithRari)['~rari']?.cookies?.()
  if (!store) {
    throw new Error(
      '[rari] cookies() is only available in server actions and server components.',
    )
  }

  return store
}
