import type { ActionRevalidationKind } from './revalidation-kind'
import {
  ActionDidNotRevalidate,
  parseActionRevalidationKind,
} from './revalidation-kind'

export interface ActionFlightRefreshDetail {
  element: unknown
  revalidationKind: ActionRevalidationKind
  revalidatedPath?: string
}

/** Flight action response: `a` is the action result, `f` is the optional refresh tree. */
export interface ActionFlightResponseShape {
  a?: unknown
  f?: unknown
}

const SKIP_REFRESH_MARKER = '~rariSkipRefresh'

function shouldSkipRefreshForActionResult(result: unknown): boolean {
  if (result == null || typeof result !== 'object' || Array.isArray(result))
    return false

  const record = result as Record<string, unknown>
  if ('redirect' in record)
    return true

  return SKIP_REFRESH_MARKER in record || '~rariFormState' in record
}

function isLikelyReactElement(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object'
    && value !== null
    && '$$typeof' in value
}

async function resolveRefreshElement(
  refreshFlight: ActionFlightResponseShape['f'],
): Promise<unknown> {
  if (refreshFlight == null || refreshFlight === '')
    return null

  const resolved = refreshFlight instanceof Promise ? await refreshFlight : refreshFlight
  if (resolved == null || resolved === '')
    return null

  return resolved
}

export function scheduleActionFlightRefresh(
  response: Response,
  flightResponse: ActionFlightResponseShape,
  actionResult: unknown,
): void {
  if (typeof globalThis.dispatchEvent !== 'function')
    return

  if (shouldSkipRefreshForActionResult(actionResult))
    return

  void (async () => {
    const refreshElement = await resolveRefreshElement(flightResponse.f)
    if (refreshElement == null)
      return

    if (!isLikelyReactElement(refreshElement))
      return

    const revalidationKind = parseActionRevalidationKind(
      response.headers.get('x-action-revalidated'),
    )

    if (revalidationKind === ActionDidNotRevalidate)
      return

    const revalidatedPath = response.headers.get('x-action-revalidated-path') ?? undefined

    const applyRefresh = () => {
      globalThis.dispatchEvent(new CustomEvent('rari:action-flight-refresh', {
        detail: {
          element: refreshElement,
          revalidationKind,
          revalidatedPath,
        } satisfies ActionFlightRefreshDetail,
      }))
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
  if (typeof globalThis.dispatchEvent !== 'function')
    return

  globalThis.dispatchEvent(new CustomEvent('rari:app-router-rerender'))
}

export {
  ActionDidNotRevalidate,
  ActionDidRevalidateDynamicOnly,
  ActionDidRevalidateStaticAndDynamic,
} from './revalidation-kind'
