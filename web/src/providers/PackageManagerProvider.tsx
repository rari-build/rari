/* eslint-disable react/no-use-context, react/no-context-provider */
'use client'

import type { ReactNode } from 'react'
import { createContext, useCallback, useContext, useState } from 'react'

export type PackageManager = 'pnpm' | 'npm' | 'yarn' | 'bun' | 'deno'

const PACKAGE_MANAGERS: readonly PackageManager[] = ['pnpm', 'npm', 'yarn', 'bun', 'deno'] as const

interface PackageManagerContextType {
  packageManager: PackageManager
  setPackageManager: (pm: PackageManager) => void
}

const PackageManagerContext = createContext<PackageManagerContextType | null>(null)

export function PackageManagerProvider({ children }: { children: ReactNode }) {
  const [packageManager, setPackageManager] = useState<PackageManager>(() => {
    if (typeof window !== 'undefined') {
      const stored = localStorage.getItem('preferred-package-manager') as PackageManager | null
      if (stored && PACKAGE_MANAGERS.includes(stored))
        return stored
    }

    return 'pnpm'
  })

  const handleSetPackageManager = useCallback((pm: PackageManager) => {
    setPackageManager(pm)
    localStorage.setItem('preferred-package-manager', pm)
  }, [])

  return (
    <PackageManagerContext.Provider value={{ packageManager, setPackageManager: handleSetPackageManager }}>
      {children}
    </PackageManagerContext.Provider>
  )
}

export function usePackageManager() {
  const context = useContext(PackageManagerContext)

  if (!context) {
    throw new Error('usePackageManager must be used within a PackageManagerProvider')
  }

  return context
}
