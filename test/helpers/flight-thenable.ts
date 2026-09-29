import type { ReactNode } from 'react'
import { castMock } from './mock-cast'

export function fulfilledFlightNode(value: ReactNode): ReactNode {
  return castMock(
    Object.assign(Promise.resolve(value), {
      status: 'fulfilled' as const,
      value,
    }),
  )
}

export function pendingFlightNode(): ReactNode {
  return castMock(
    Object.assign(new Promise<ReactNode>(() => {}), {
      status: 'pending' as const,
    }),
  )
}
