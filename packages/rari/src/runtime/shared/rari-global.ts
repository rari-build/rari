import type { ComponentInfo, GlobalWithRari, WindowWithRari } from './types'

export type RariGlobalBag = GlobalWithRari['~rari']

export function getRariGlobalRoot(): GlobalWithRari {
  return globalThis as unknown as GlobalWithRari
}

export function getRariGlobal(): RariGlobalBag {
  const root = getRariGlobalRoot()
  root['~rari'] ??= {} as RariGlobalBag
  return root['~rari']
}

export function getClientComponents(): Record<string, ComponentInfo> {
  const root = getRariGlobalRoot()
  root['~clientComponents'] ??= {}
  return root['~clientComponents']
}

export function getClientComponentPaths(): Record<string, string> {
  const root = getRariGlobalRoot()
  root['~clientComponentPaths'] ??= {}
  return root['~clientComponentPaths']
}

export function getClientComponentNames(): Record<string, string> {
  const root = getRariGlobalRoot()
  root['~clientComponentNames'] ??= {}
  return root['~clientComponentNames']
}

export function getRariWindow(): WindowWithRari | null {
  if (typeof window === 'undefined')
    return null

  return window as unknown as WindowWithRari
}

export function getRariWindowBag(): RariGlobalBag | null {
  const win = getRariWindow()
  if (!win)
    return null
  win['~rari'] ??= getRariGlobal()
  return win['~rari']
}
