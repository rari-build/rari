'use client'

import { revalidateUseCacheE2eTag } from './actions'

export default function RevalidateTagButton() {
  return (
    <button
      data-testid="revalidate-tag"
      type="button"
      onClick={() => {
        void revalidateUseCacheE2eTag()
      }}
    >
      Revalidate tag
    </button>
  )
}
