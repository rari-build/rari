import type { ActionRevalidationKind } from './revalidation-kind'
import { isRecord } from '@/shared/utils/type-guards'
import { ActionDidNotRevalidate, parseActionRevalidationKind } from './revalidation-kind'

export interface ActionFlightRefreshDetail {
  element: unknown
  revalidationKind: ActionRevalidationKind
  revalidatedPath?: string
}

export interface ActionFlightResponseShape {
  readonly a?: unknown
  readonly f?: unknown
}

const SKIP_REFRESH_MARKER = '~rariSkipRefresh'

function shouldSkipRefreshForActionResult(result: unknown): boolean {
  if (!isRecord(result)) return false

  if ('redirect' in result) return true

  return SKIP_REFRESH_MARKER in result || '~rariFormState' in result
}

function isLikelyReactElement(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && '$$typeof' in value
}

function isFlightThenable(value: unknown): value is PromiseLike<unknown> {
  return isRecord(value) && typeof value.then === 'function'
}

function isActionRefreshRoot(value: unknown): boolean {
  return isLikelyReactElement(value) || isFlightThenable(value)
}

function resolveRefreshElement(refreshFlight: ActionFlightResponseShape['f']): unknown {
  if (refreshFlight == null || refreshFlight === '') return null

  if (refreshFlight instanceof Promise) {
    return refreshFlight.then((resolved: unknown) => {
      if (resolved == null || resolved === '') return null
      return resolved
    })
  }

  return refreshFlight
}

export function scheduleActionFlightRefresh(
  response: Response,
  flightResponse: ActionFlightResponseShape,
  actionResult: unknown,
): void {
  if (typeof globalThis.dispatchEvent !== 'function') return

  if (shouldSkipRefreshForActionResult(actionResult)) return

  void (async () => {
    const pending = resolveRefreshElement(flightResponse.f)
    const refreshElement: unknown = pending instanceof Promise ? await pending : pending
    if (refreshElement == null) return

    if (!isActionRefreshRoot(refreshElement)) return

    const revalidationKind = parseActionRevalidationKind(
      response.headers.get('x-action-revalidated'),
    )

    if (revalidationKind === ActionDidNotRevalidate) return

    const revalidatedPath = response.headers.get('x-action-revalidated-path') ?? undefined

    const applyRefresh = () => {
      globalThis.dispatchEvent(
        new CustomEvent('rari:action-flight-refresh', {
          detail: {
            element: refreshElement,
            revalidationKind,
            revalidatedPath,
          } satisfies ActionFlightRefreshDetail,
        }),
      )
    }

    if (typeof requestAnimationFrame === 'function') {
      requestAnimationFrame(() => {
        requestAnimationFrame(applyRefresh)
      })
      return
    }

    queueMicrotask(applyRefresh)
  })().catch((error: unknown) => {
    console.error(
      '[rari] Action flight refresh failed:',
      error instanceof Error ? error.message : String(error),
    )
  })
}

export function refreshRouter(): void {
  if (typeof globalThis.dispatchEvent !== 'function') return

  globalThis.dispatchEvent(new CustomEvent('rari:app-router-rerender'))
}

export {
  ActionDidNotRevalidate,
  ActionDidRevalidateDynamicOnly,
  ActionDidRevalidateStaticAndDynamic,
} from './revalidation-kind'
