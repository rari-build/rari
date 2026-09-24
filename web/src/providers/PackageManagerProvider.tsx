'use client'

import type { ReactNode } from 'react'
import type { PackageManager } from './usePackageManager'
import { useCallback, useState } from 'react'
import { PackageManagerContext } from './usePackageManager'

function isPackageManager(value: string): value is PackageManager {
  return value === 'pnpm' || value === 'npm' || value === 'yarn' || value === 'bun'
}

export function PackageManagerProvider({ children }: Readonly<{ children: ReactNode }>) {
  const [packageManager, setPackageManager] = useState<PackageManager>(() => {
    if (typeof window !== 'undefined') {
      try {
        const stored = localStorage.getItem('preferred-package-manager')
        if (stored !== null && isPackageManager(stored)) return stored
      } catch {
        // localStorage access failed, fall back to default
      }
    }

    return 'pnpm'
  })

  const handleSetPackageManager = useCallback((pm: PackageManager) => {
    setPackageManager(pm)
    try {
      localStorage.setItem('preferred-package-manager', pm)
    } catch {
      // localStorage write failed, but state is still updated
    }
  }, [])

  return (
    <PackageManagerContext value={{ packageManager, setPackageManager: handleSetPackageManager }}>
      {children}
    </PackageManagerContext>
  )
}
