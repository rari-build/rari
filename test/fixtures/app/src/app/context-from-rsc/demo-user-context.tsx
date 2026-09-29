'use client'

import { createContext, use } from 'react'

export interface DemoUser {
  readonly name: string
  readonly role: string
}

export const DemoUserContext = createContext<DemoUser | null>(null)

export function DemoUserLabel() {
  const user = use(DemoUserContext)
  if (user == null) throw new Error('DemoUserContext is missing')

  return (
    <p data-testid="context-user-label">
      Signed in as {user.name} ({user.role})
    </p>
  )
}
