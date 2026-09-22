'use client'

import type { ReactNode } from 'react'
import { use } from 'react'
import { isFlightThenable } from '@/shared/utils/type-guards'

type FulfilledThenable<T> = PromiseLike<T> & {
  status: 'fulfilled'
  value: T
}

function asFulfilledThenable<T>(value: T): FulfilledThenable<T> {
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion
  const thenable = Promise.resolve(value) as unknown as FulfilledThenable<T>
  thenable.status = 'fulfilled'
  thenable.value = value
  return thenable
}

function FlightThenable({ thenable }: { readonly thenable: PromiseLike<ReactNode> }): ReactNode {
  return use(thenable)
}

export function FlightOutlet({
  content,
}: {
  readonly content: ReactNode | PromiseLike<ReactNode>
}): ReactNode {
  const thenable = isFlightThenable<ReactNode>(content) ? content : asFulfilledThenable(content)
  return <FlightThenable thenable={thenable} />
}
