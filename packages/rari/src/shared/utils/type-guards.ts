import type { ComponentType, ReactElement, ReactNode } from 'react'
import type { ProxyModule } from '@/proxy/http/types'

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function isErrnoException(error: unknown): error is NodeJS.ErrnoException {
  return error instanceof Error && 'code' in error
}

export function isFunction(value: unknown): value is (...args: readonly unknown[]) => unknown {
  return typeof value === 'function'
}

export function parseJsonRecord(text: string): Record<string, unknown> | null {
  try {
    const parsed: unknown = JSON.parse(text)
    return isRecord(parsed) ? parsed : null
  } catch {
    return null
  }
}

export function readPackageManagerFieldFromRecord(
  pkg: Readonly<Record<string, unknown>>,
): 'pnpm' | 'yarn' | 'bun' | 'npm' | null {
  const packageManager = pkg.packageManager
  if (typeof packageManager !== 'string') return null

  if (packageManager.startsWith('pnpm')) return 'pnpm'
  if (packageManager.startsWith('yarn')) return 'yarn'
  if (packageManager.startsWith('bun')) return 'bun'
  if (packageManager.startsWith('npm')) return 'npm'

  return null
}

export function readViteBinFromPackageRecord(
  pkg: Readonly<Record<string, unknown>>,
): 'vp' | 'vite' | null {
  const dependencies = isRecord(pkg.dependencies) ? pkg.dependencies : {}
  const devDependencies = isRecord(pkg.devDependencies) ? pkg.devDependencies : {}
  const deps = { ...dependencies, ...devDependencies }

  if (typeof deps['vite-plus'] === 'string') return 'vp'
  if (typeof deps.vite === 'string') return 'vite'

  return null
}

export function isProxyModule(module: unknown): module is ProxyModule {
  if (!isRecord(module)) return false

  const proxy = module.proxy
  const defaultExport = module.default
  const hasProxy = isFunction(proxy) || isFunction(defaultExport)

  if (!hasProxy) return false

  if (module.config !== undefined && !isRecord(module.config)) return false

  return true
}

export function getCustomEventDetail<T>(
  event: Event,
  predicate: (detail: unknown) => detail is T,
): T | undefined {
  if (!(event instanceof CustomEvent)) return undefined

  const detail: unknown = event.detail
  if (!predicate(detail)) return undefined

  return detail
}

export function getErrnoCode(error: unknown): string | undefined {
  return isErrnoException(error) ? error.code : undefined
}

export function parseJsonArrayRecord(
  text: string,
  key: string,
): Array<Record<string, unknown>> | null {
  const parsed = parseJsonRecord(text)
  if (!parsed) return null

  const value = parsed[key]
  if (!Array.isArray(value)) return null

  return value.filter(isRecord)
}

export function isAliasArray(
  value: unknown,
): value is Array<{ find: string | RegExp; replacement: string }> {
  if (!Array.isArray(value)) return false

  return value.every(entry => {
    if (!isRecord(entry)) return false

    const { find, replacement } = entry
    return (typeof find === 'string' || find instanceof RegExp) && typeof replacement === 'string'
  })
}

export function isComponentType(value: unknown): value is ComponentType<any> {
  return (
    typeof value === 'function' || (isRecord(value) && ('$$typeof' in value || 'render' in value))
  )
}

export function isReactElementLike(
  value: Readonly<Record<string, unknown>>,
): value is { type: unknown; props?: Record<string, unknown> } {
  return 'type' in value
}

export function isFlightImportTuple(
  value: unknown,
): value is [string, unknown, string | undefined, ...unknown[]] {
  return Array.isArray(value) && typeof value[0] === 'string'
}

export function isClientReferenceType(type: unknown): boolean {
  return isRecord(type) && type.$$typeof === Symbol.for('react.client.reference')
}

export function hasClientReferenceId(type: unknown): type is { $$id: string } {
  return isRecord(type) && typeof type.$$id === 'string'
}

export function getReactElementProps(element: ReactElement): { children?: ReactNode } {
  return element.props as { children?: ReactNode } // oxlint-disable-line typescript/no-unsafe-type-assertion -- ReactElement props are intentionally loose
}

export function aliasEntriesFromRecord(
  aliases: Readonly<Record<string, unknown>>,
): Array<{ find: string; replacement: string }> {
  return Object.entries(aliases).flatMap(([key, value]) =>
    typeof value === 'string' ? [{ find: key, replacement: value }] : [],
  )
}

export function isHistoryState(value: unknown): value is {
  route: string
  navigationId: number
  scrollPosition?: { x: number; y: number }
  timestamp: number
  key: string
} {
  return (
    isRecord(value) &&
    typeof value.route === 'string' &&
    typeof value.navigationId === 'number' &&
    typeof value.timestamp === 'number' &&
    typeof value.key === 'string'
  )
}

export function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every(item => typeof item === 'string')
}

export function isStaticParamsArray(
  value: unknown,
): value is Array<Record<string, string | string[]>> {
  if (!Array.isArray(value)) return false

  return value.every(entry => {
    if (!isRecord(entry)) return false

    return Object.values(entry).every(item => typeof item === 'string' || isStringArray(item))
  })
}

export function warnInvalidStaticParams(source: string): void {
  console.warn(
    `[rari] generateStaticParams() in ${source} returned invalid params. ` +
      `Expected Array<Record<string, string | string[]>> (e.g. [{ slug: "post" }]). ` +
      `Static routes for this page will not be generated.`,
  )
}
