'use client'

import React, { useCallback, useEffect, useRef } from 'react'
import { ClientRouter } from '../router/ClientRouter'
import { StatePreserver } from '../router/StatePreserver'
import { AppRouterProvider } from './AppRouterProvider'

export interface NavigationProviderProps {
  children: React.ReactNode
  initialRoute: string
  initialPayload?: any
}

export function NavigationProvider({
  children,
  initialRoute,
  initialPayload,
}: NavigationProviderProps) {
  const statePreserverRef = useRef<StatePreserver>(new StatePreserver())

  const handleNavigate = useCallback((detail: any) => {
    const hasPreservedState = statePreserverRef.current.hasState(detail.to)
    if (hasPreservedState) {
      requestAnimationFrame(() => {
        statePreserverRef.current.restoreState(detail.to)
      })
    }
  }, [])

  useEffect(() => {
    if (typeof window === 'undefined') {
      return
    }

    const handleBeforeNavigate = (event: Event) => {
      const customEvent = event as CustomEvent
      const detail = customEvent.detail

      statePreserverRef.current.captureState(detail.from)
    }

    window.addEventListener('rari:navigate', handleBeforeNavigate)

    return () => {
      window.removeEventListener('rari:navigate', handleBeforeNavigate)
    }
  }, [])

  return (
    <ClientRouter initialRoute={initialRoute}>
      <AppRouterProvider initialPayload={initialPayload} onNavigate={handleNavigate}>
        {children}
      </AppRouterProvider>
    </ClientRouter>
  )
}
