'use client'

import type { ReactNode } from 'react'
import { use } from 'react'
import { isFlightThenable } from '@/shared/utils/type-guards'

function FlightThenable({ thenable }: { readonly thenable: PromiseLike<ReactNode> }): ReactNode {
  return use(thenable)
}

export function FlightOutlet({
  content,
}: {
  readonly content: ReactNode | PromiseLike<ReactNode>
}): ReactNode {
  if (!isFlightThenable<ReactNode>(content)) return content
  return <FlightThenable thenable={content} />
}
