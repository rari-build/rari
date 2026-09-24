import {
  createFromFetch,
  createTemporaryReferenceSet,
  encodeReply,
} from 'virtual:react-flight-client'
import { isError, isRecord } from '@/shared/utils/type-guards'
import { serializeRouterState } from '../flight/serialize-router-state'
import { scheduleActionFlightRefresh } from './flight-refresh'

interface ActionFlightResponse {
  a: unknown
  f?: unknown
}

const ACTION_REQUEST_TIMEOUT_MS = 30_000

function stripInternalActionMetadata(result: unknown): unknown {
  if (!isRecord(result)) return result

  const { '~rariSkipRefresh': _skipRefresh, ...rest } = result
  return rest
}

function actionPostUrl(): string {
  if (typeof window !== 'undefined') return window.location.pathname + window.location.search

  return '/'
}

const ALLOWED_REDIRECT_PROTOCOLS = new Set(['http:', 'https:'])

function applyRedirect(redirect: string) {
  if (typeof window === 'undefined') return

  try {
    const absoluteRedirect = new URL(redirect, window.location.href)
    if (!ALLOWED_REDIRECT_PROTOCOLS.has(absoluteRedirect.protocol)) return

    if (absoluteRedirect.href !== window.location.href) window.location.href = absoluteRedirect.href
  } catch {
    // Ignore malformed redirect targets.
  }
}

export async function callServer(id: string, args: readonly unknown[]): Promise<unknown> {
  const temporaryReferences = createTemporaryReferenceSet()
  const encoded = await encodeReply(args, { temporaryReferences })
  const { headers, body } = buildActionRequestBody(encoded)
  headers.Accept = 'text/x-component'
  headers['rsc-action-id'] = id
  headers['rari-router-state'] = serializeRouterState()

  const response = await fetchActionResponse(id, headers, body)

  const redirectResult = tryActionRedirect(response)
  if (redirectResult != null) return redirectResult

  await assertFlightActionResponse(id, response)

  const flightResponse = await createFromFetch<ActionFlightResponse>(Promise.resolve(response), {
    callServer,
    temporaryReferences,
  })

  const actionResult: unknown = flightResponse.a
  const resolvedActionResult: unknown =
    actionResult instanceof Promise ? await actionResult : actionResult

  scheduleActionFlightRefresh(response, flightResponse, resolvedActionResult)

  return stripInternalActionMetadata(resolvedActionResult)
}

function buildActionRequestBody(encoded: string | FormData): {
  headers: Record<string, string>
  body: BodyInit
} {
  const headers: Record<string, string> = {}
  if (typeof encoded === 'string') {
    headers['Content-Type'] = 'text/plain;charset=UTF-8'
    return { headers, body: encoded }
  }
  return { headers, body: encoded }
}

async function fetchActionResponse(
  id: string,
  headers: Readonly<Record<string, string>>,
  body: BodyInit,
): Promise<Response> {
  try {
    return await fetch(actionPostUrl(), {
      method: 'POST',
      headers,
      body,
      signal: AbortSignal.timeout(ACTION_REQUEST_TIMEOUT_MS),
    })
  } catch (error) {
    if (
      (error instanceof DOMException && error.name === 'TimeoutError') ||
      (isError(error) && error.name === 'AbortError')
    ) {
      throw new Error(`Server action "${id}" timed out after ${ACTION_REQUEST_TIMEOUT_MS}ms`)
    }
    throw error
  }
}

function tryActionRedirect(response: Response): { redirect: string } | null {
  const redirectHeader = response.headers.get('x-action-redirect')
  if (redirectHeader == null || redirectHeader === '') return null
  const [location = ''] = redirectHeader.split(';')
  if (location !== '') applyRedirect(location)
  return { redirect: location }
}

async function assertFlightActionResponse(id: string, response: Response): Promise<void> {
  const contentTypeHeader = response.headers.get('content-type')
  const contentType = contentTypeHeader != null && contentTypeHeader !== '' ? contentTypeHeader : ''
  if (contentType.startsWith('text/x-component')) return

  const message =
    response.status >= 400 && contentType.startsWith('text/plain')
      ? await response.text().catch(() => response.statusText)
      : `Server action "${id}" failed with status ${response.status}: ${response.statusText}`

  throw new Error(message)
}
