import {
  createTemporaryReferenceSet,
  encodeReply,
} from 'react-server-dom-webpack/client'

import { encodeFormData } from './encode-form-data'

export async function encodeCacheKeyParts(parts: readonly unknown[]): Promise<string> {
  const temporaryReferences = createTemporaryReferenceSet()
  const encoded = await encodeReply(parts, { temporaryReferences })
  if (typeof encoded === 'string')
    return encoded

  return encodeFormData(encoded)
}
