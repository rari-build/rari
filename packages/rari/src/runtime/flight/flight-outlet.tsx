'use client'

import type { ReactNode } from 'react'
import type { Thenable } from 'virtual:react-flight-client'
import { use } from 'react'
import { isRecord } from '@/shared/utils/type-guards'

function isFlightThenable(value: unknown): value is Thenable<ReactNode> {
  return isRecord(value) && typeof value.then === 'function'
}

function FlightThenable({ thenable }: { readonly thenable: Thenable<ReactNode> }): ReactNode {
  return use(thenable as PromiseLike<ReactNode>)
}

export function FlightOutlet({
  content,
}: {
  readonly content: ReactNode | Thenable<ReactNode>
}): ReactNode {
  if (!isFlightThenable(content)) return content
  return <FlightThenable thenable={content} />
}
