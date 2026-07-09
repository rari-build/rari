import {
  createFromFetch,
  createTemporaryReferenceSet,
  encodeReply,
} from 'virtual:react-flight-client'
import { scheduleActionFlightRefresh } from './action-flight-refresh'
import { serializeRouterState } from './router-state'

interface ActionFlightResponse {
  a: unknown
  f?: unknown
}

const ACTION_REQUEST_TIMEOUT_MS = 30_000

function stripInternalActionMetadata<T>(result: T): T {
  if (!result || typeof result !== 'object' || Array.isArray(result))
    return result

  const record = { ...(result as Record<string, unknown>) }
  delete record['~rariSkipRefresh']
  return record as T
}

function actionPostUrl(): string {
  if (typeof window !== 'undefined')
    return window.location.pathname + window.location.search

  return '/'
}

const ALLOWED_REDIRECT_PROTOCOLS = new Set(['http:', 'https:'])

function applyRedirect(redirect: string) {
  if (typeof window === 'undefined')
    return

  try {
    const absoluteRedirect = new URL(redirect, window.location.href)
    if (!ALLOWED_REDIRECT_PROTOCOLS.has(absoluteRedirect.protocol))
      return

    if (absoluteRedirect.href !== window.location.href)
      window.location.href = absoluteRedirect.href
  }
  catch {
    // Ignore malformed redirect targets.
  }
}

export async function callServer(id: string, args: unknown[]): Promise<unknown> {
  const temporaryReferences = createTemporaryReferenceSet()
  const encoded = await encodeReply(args, { temporaryReferences })
  const headers: Record<string, string> = {
    'Accept': 'text/x-component',
    'rsc-action-id': id,
    'rari-router-state': serializeRouterState(),
  }

  let body: BodyInit
  if (typeof encoded === 'string') {
    headers['Content-Type'] = 'text/plain;charset=UTF-8'
    body = encoded
  }
  else {
    body = encoded
  }

  const controller = new AbortController()
  const timeoutId = setTimeout(() => controller.abort(), ACTION_REQUEST_TIMEOUT_MS)

  let response: Response
  try {
    response = await fetch(actionPostUrl(), {
      method: 'POST',
      headers,
      body,
      signal: controller.signal,
    })
  }
  catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      throw new Error(`Server action "${id}" timed out after ${ACTION_REQUEST_TIMEOUT_MS}ms`)
    }

    throw error
  }
  finally {
    clearTimeout(timeoutId)
  }

  const redirectHeader = response.headers.get('x-action-redirect')
  if (redirectHeader) {
    const [location] = redirectHeader.split(';')
    if (location)
      applyRedirect(location)

    return { redirect: location }
  }

  const contentType = response.headers.get('content-type') || ''
  const isFlightResponse = contentType.startsWith('text/x-component')

  if (!isFlightResponse) {
    const message = response.status >= 400 && contentType.startsWith('text/plain')
      ? await response.text().catch(() => response.statusText)
      : `Server action "${id}" failed with status ${response.status}: ${response.statusText}`

    throw new Error(message)
  }

  const flightResponse = await createFromFetch(Promise.resolve(response), {
    callServer: callServer as <A, R>(id: string, args: A) => Promise<R>,
    temporaryReferences,
  }) as ActionFlightResponse

  const actionResult = flightResponse.a
  const resolvedActionResult = actionResult instanceof Promise ? await actionResult : actionResult

  scheduleActionFlightRefresh(response, flightResponse, resolvedActionResult)

  return stripInternalActionMetadata(resolvedActionResult)
}
