import {
  createFromFetch,
  createTemporaryReferenceSet,
  encodeReply,
} from 'virtual:react-flight-client'
import { isRecord } from '@/shared/utils/type-guards'
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
  const headers: Record<string, string> = {
    'Accept': 'text/x-component',
    'rsc-action-id': id,
    'rari-router-state': serializeRouterState(),
  }

  let body: BodyInit
  if (typeof encoded === 'string') {
    headers['Content-Type'] = 'text/plain;charset=UTF-8'
    body = encoded
  } else {
    body = encoded
  }

  const controller = new AbortController()
  const timeoutId = setTimeout(() => {
    controller.abort()
  }, ACTION_REQUEST_TIMEOUT_MS)

  let response: Response
  try {
    response = await fetch(actionPostUrl(), {
      method: 'POST',
      headers,
      body,
      signal: controller.signal,
    })
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      throw new Error(`Server action "${id}" timed out after ${ACTION_REQUEST_TIMEOUT_MS}ms`)
    }

    throw error
  } finally {
    clearTimeout(timeoutId)
  }

  const redirectHeader = response.headers.get('x-action-redirect')
  if (redirectHeader != null && redirectHeader !== '') {
    const [location = ''] = redirectHeader.split(';')
    if (location !== '') applyRedirect(location)

    return { redirect: location }
  }

  const contentTypeHeader = response.headers.get('content-type')
  const contentType = contentTypeHeader != null && contentTypeHeader !== '' ? contentTypeHeader : ''
  const isFlightResponse = contentType.startsWith('text/x-component')

  if (!isFlightResponse) {
    const message =
      response.status >= 400 && contentType.startsWith('text/plain')
        ? await response.text().catch(() => response.statusText)
        : `Server action "${id}" failed with status ${response.status}: ${response.statusText}`

    throw new Error(message)
  }

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
