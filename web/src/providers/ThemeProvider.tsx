'use client'

import type { ReactNode } from 'react'
import { createContext, use, useCallback, useEffect, useMemo, useState } from 'react'

export type ThemePreference = 'light' | 'dark' | 'system'
export type ResolvedTheme = 'light' | 'dark'

const THEME_STORAGE_KEY = 'preferred-theme'
function isThemePreference(value: string): value is ThemePreference {
  return value === 'light' || value === 'dark' || value === 'system'
}

interface ThemeContextType {
  theme: ThemePreference
  resolvedTheme: ResolvedTheme
  setTheme: (theme: ThemePreference) => void
}

const ThemeContext = createContext<ThemeContextType | null>(null)

function getSystemTheme(): ResolvedTheme {
  if (typeof window === 'undefined') return 'dark'

  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function resolveTheme(preference: ThemePreference): ResolvedTheme {
  return preference === 'system' ? getSystemTheme() : preference
}

function applyThemeClass(resolved: ResolvedTheme) {
  const root = document.documentElement
  root.classList.toggle('light', resolved === 'light')
  root.classList.toggle('dark', resolved === 'dark')
}

function readStoredTheme(): ThemePreference {
  if (typeof window === 'undefined') return 'system'

  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY)
    if (stored !== null && isThemePreference(stored)) return stored
  } catch {
    // localStorage access failed
  }

  return 'system'
}

export function ThemeProvider({ children }: Readonly<{ children: ReactNode }>) {
  const [theme, setTheme] = useState<ThemePreference>(readStoredTheme)
  const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>(() =>
    resolveTheme(readStoredTheme()),
  )

  const handleSetTheme = useCallback((next: ThemePreference) => {
    setTheme(next)
    const resolved = resolveTheme(next)
    setResolvedTheme(resolved)
    applyThemeClass(resolved)
    try {
      localStorage.setItem(THEME_STORAGE_KEY, next)
    } catch {
      // localStorage write failed
    }
  }, [])

  useEffect(() => {
    applyThemeClass(resolveTheme(theme))
  }, [theme])

  useEffect(() => {
    if (theme !== 'system') return undefined

    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const onChange = () => {
      const resolved = getSystemTheme()
      setResolvedTheme(resolved)
      applyThemeClass(resolved)
    }

    media.addEventListener('change', onChange)
    return () => {
      media.removeEventListener('change', onChange)
    }
  }, [theme])

  const value = useMemo(
    () => ({ theme, resolvedTheme, setTheme: handleSetTheme }),
    [theme, resolvedTheme, handleSetTheme],
  )

  return <ThemeContext value={value}>{children}</ThemeContext>
}

// eslint-disable-next-line react-refresh/only-export-components
export function useTheme() {
  const context = use(ThemeContext)

  if (!context) throw new Error('useTheme must be used within a ThemeProvider')

  return context
}
