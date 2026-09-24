'use client'

import { createContext, use } from 'react'

export type PackageManager = 'pnpm' | 'npm' | 'yarn' | 'bun'

interface PackageManagerContextType {
  packageManager: PackageManager
  setPackageManager: (pm: PackageManager) => void
}

export const PackageManagerContext = createContext<PackageManagerContextType | null>(null)

export function usePackageManager() {
  const context = use(PackageManagerContext)

  if (!context) {
    throw new Error('usePackageManager must be used within a PackageManagerProvider')
  }

  return context
}
