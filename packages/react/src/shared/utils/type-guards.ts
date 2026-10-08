import type { ComponentType } from 'react'
import { isRecord, isStringArray } from '@rari/core/utils/type-guards'

export * from '@rari/core/utils/type-guards'

export function isFlightThenable<T = unknown>(value: unknown): value is PromiseLike<T> {
  return isRecord(value) && typeof value.then === 'function'
}

export function isComponentType(value: unknown): value is ComponentType<any> {
  return (
    typeof value === 'function' || (isRecord(value) && ('$$typeof' in value || 'render' in value))
  )
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
