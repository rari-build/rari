import { getAllUseCacheStorages } from '@/runtime/storage/registry'
import { invalidateUseCacheViaOp } from '@/runtime/storage/remote-ops'
import {
  createRegistryBackedDelete,
  invalidateUseCacheTag,
} from './cache-tag-registry'

export async function invalidateUseCacheEntries(input: {
  tag?: string
  path?: string
}): Promise<void> {
  await invalidateUseCacheViaOp(input)
}

export async function invalidateUseCacheByTag(tag: string): Promise<number> {
  const deleteKey = createRegistryBackedDelete(getAllUseCacheStorages())
  return invalidateUseCacheTag(tag, deleteKey)
}
