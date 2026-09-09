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
    <p className="text-lg text-gray-900">
      Signed in as <span className="font-semibold">{user.name}</span> ({user.role})
    </p>
  )
}
