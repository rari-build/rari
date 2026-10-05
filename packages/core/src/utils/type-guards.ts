export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function isErrnoException(error: unknown): error is NodeJS.ErrnoException {
  return isError(error) && 'code' in error
}

export function isError(value: unknown): value is Error {
  return typeof Error.isError === 'function' ? Error.isError(value) : value instanceof Error
}

export function asError(value: unknown): Error | undefined {
  return isError(value) ? value : undefined
}

export function toError(value: unknown): Error {
  return asError(value) ?? new Error(String(value))
}

export function errorMessage(value: unknown, fallback = 'Unknown error'): string {
  if (isError(value)) return value.message
  if (typeof value === 'string') return value
  return fallback
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

export function aliasEntriesFromRecord(
  aliases: Readonly<Record<string, unknown>>,
): Array<{ find: string; replacement: string }> {
  return Object.entries(aliases).flatMap(([key, value]) =>
    typeof value === 'string' ? [{ find: key, replacement: value }] : [],
  )
}

export function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every(item => typeof item === 'string')
}
