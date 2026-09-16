'use server'

import { revalidateTag } from '@rari/use-cache/runtime/cache-wrapper'

export async function revalidateUseCacheE2eTag(): Promise<void> {
  await revalidateTag('use-cache-revalidate-e2e')
}
