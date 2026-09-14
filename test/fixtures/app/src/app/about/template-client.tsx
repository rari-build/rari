'use client'

import type { ReactNode } from 'react'
import { useRef, useSyncExternalStore } from 'react'

function createMountId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function')
    return crypto.randomUUID()
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
}

function useMountId(): string {
  const clientIdRef = useRef<string | null>(null)

  return useSyncExternalStore(
    () => () => {},
    () => {
      clientIdRef.current ??= createMountId()
      return clientIdRef.current
    },
    () => 'ssr',
  )
}

export function AboutTemplateClient({ children }: Readonly<{ children: ReactNode }>) {
  const mountCount = useMountId()

  return (
    <div data-testid="about-template" data-mount-count={mountCount}>
      {children}
    </div>
  )
}
