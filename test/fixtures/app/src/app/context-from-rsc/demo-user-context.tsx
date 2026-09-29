'use client'

import { createContext, use, useSyncExternalStore } from 'react'

export interface DemoUser {
  readonly name: string
  readonly role: string
}

export const DemoUserContext = createContext<DemoUser | null>(null)

function subscribe(): () => void {
  return () => {}
}

export function DemoUserLabel() {
  const user = use(DemoUserContext)
  const hydrated = useSyncExternalStore(
    subscribe,
    () => true,
    () => false,
  )

  if (user == null) throw new Error('DemoUserContext is missing')

  return (
    <p data-testid="context-user-label" data-hydrated={hydrated ? 'true' : 'false'}>
      Signed in as {user.name} ({user.role})
    </p>
  )
}
