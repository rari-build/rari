import type { ComponentInfo, GlobalWithRari, RariGlobalBag, WindowWithRari } from './types'

export type { RariGlobalBag }

export function getRariGlobalRoot(): GlobalWithRari {
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- runtime global bag lives on globalThis
  return globalThis as unknown as GlobalWithRari
}

export function getRariGlobal(): RariGlobalBag {
  const root = getRariGlobalRoot()
  let bag = root['~rari']
  if (bag == null) {
    bag = {}
    root['~rari'] = bag
  }

  return bag
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
  if (typeof window === 'undefined') return null

  return window
}

export function getRariWindowBag(): RariGlobalBag | null {
  const win = getRariWindow()
  if (!win) return null
  win['~rari'] ??= getRariGlobal()
  return win['~rari']
}
