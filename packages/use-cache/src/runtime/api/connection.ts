import { markUseCacheDynamicContext } from '../cache-dynamic-context'

export async function connection(): Promise<void> {
  markUseCacheDynamicContext()
}
