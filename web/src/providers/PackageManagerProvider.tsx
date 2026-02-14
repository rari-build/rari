'use client'

import type { ReactNode } from 'react'
import { createContext, use, useCallback, useState } from 'react'

export type PackageManager = 'pnpm' | 'npm' | 'yarn' | 'bun' | 'deno'

export const PACKAGE_MANAGERS: readonly PackageManager[] = ['pnpm', 'npm', 'yarn', 'bun', 'deno'] as const

interface PackageManagerContextType {
  packageManager: PackageManager
  setPackageManager: (pm: PackageManager) => void
}

const PackageManagerContext = createContext<PackageManagerContextType>({
  packageManager: 'pnpm',
  setPackageManager: () => {},
})

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
    <PackageManagerContext value={{ packageManager, setPackageManager: handleSetPackageManager }}>
      {children}
    </PackageManagerContext>
  )
}

export function usePackageManager() {
  return use(PackageManagerContext)
}
