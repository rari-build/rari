import { createHash } from 'node:crypto'

export function contentHash(value: string | Readonly<NodeJS.ArrayBufferView>, length = 8): string {
  return createHash('sha256').update(value).digest('hex').slice(0, length)
}
