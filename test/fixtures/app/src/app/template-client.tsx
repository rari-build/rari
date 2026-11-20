'use client'

import type { ReactNode } from 'react'
import { useRef, useSyncExternalStore, ViewTransition } from 'react'
import { navEnterExit } from './page-transition'

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

export function RootTemplateClient({ children }: Readonly<{ children: ReactNode }>) {
  const mountCount = useMountId()

  return (
    <ViewTransition enter={navEnterExit} exit={navEnterExit} default="none">
      <div data-testid="root-template" data-mount-count={mountCount}>
        <div data-testid="root-template-children">{children}</div>
      </div>
    </ViewTransition>
  )
}
