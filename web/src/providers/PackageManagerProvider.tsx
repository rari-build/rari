/* eslint-disable react/no-use-context, react/no-context-provider */
'use client'

import type { ReactNode } from 'react'
import { createContext, useCallback, useContext, useState } from 'react'

export type PackageManager = 'pnpm' | 'npm' | 'yarn' | 'bun' | 'deno'

const PACKAGE_MANAGERS: readonly PackageManager[] = ['pnpm', 'npm', 'yarn', 'bun', 'deno']

interface PackageManagerContextType {
  packageManager: PackageManager
  setPackageManager: (pm: PackageManager) => void
}

const PackageManagerContext = createContext<PackageManagerContextType | null>(null)

export function PackageManagerProvider({ children }: { children: ReactNode }) {
  const [packageManager, setPackageManager] = useState<PackageManager>(() => {
    if (typeof window !== 'undefined') {
      try {
        const stored = localStorage.getItem('preferred-package-manager') as PackageManager | null
        if (stored && PACKAGE_MANAGERS.includes(stored))
          return stored
      }
      catch {
        // localStorage access failed, fall back to default
      }
    }

    return 'pnpm'
  })

  const handleSetPackageManager = useCallback((pm: PackageManager) => {
    setPackageManager(pm)
    try {
      localStorage.setItem('preferred-package-manager', pm)
    }
    catch {
      // localStorage write failed, but state is still updated
    }
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
