import { markUseCacheDynamicContext } from '@/runtime/cache-dynamic-context'

export async function connection(): Promise<void> {
  markUseCacheDynamicContext()
  return Promise.resolve()
}
