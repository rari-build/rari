'use client'

import type { ReactNode } from 'react'
import { use, useMemo } from 'react'
import { isFlightThenable } from '@/shared/utils/type-guards'

function FlightThenable({ thenable }: { readonly thenable: PromiseLike<ReactNode> }): ReactNode {
  return use(thenable)
}

export function FlightOutlet({
  content,
}: {
  readonly content: ReactNode | PromiseLike<ReactNode>
}): ReactNode {
  const thenable = useMemo((): PromiseLike<ReactNode> => {
    if (isFlightThenable<ReactNode>(content)) return content
    return Promise.resolve(content)
  }, [content])

  return <FlightThenable thenable={thenable} />
}
