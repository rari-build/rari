import { internals } from 'ext:core/mod.js'
import {
  applyToGlobal,
  lazyExtScript,
  propNonEnumerableLazyLoaded,
  writeable,
} from 'ext:init_utilities/utilities.ts'

const lazyHeaders = lazyExtScript<DenoFetchHeadersModule>('ext:deno_fetch/20_headers.js')
const lazyFormData = lazyExtScript<DenoFetchFormDataModule>('ext:deno_fetch/21_formdata.js')
const lazyHttpClient = lazyExtScript<DenoFetchHttpClientModule>('ext:deno_fetch/22_http_client.js')
const lazyRequest = lazyExtScript<DenoFetchRequestModule>('ext:deno_fetch/23_request.js')
const lazyResponse = lazyExtScript<DenoFetchResponseModule>('ext:deno_fetch/23_response.js')
const lazyFetch = lazyExtScript<DenoFetchWasmModule>('ext:deno_fetch/26_fetch.js')
const lazyEventSource = lazyExtScript<DenoFetchEventSourceModule>(
  'ext:deno_fetch/27_eventsource.js',
)

let fetchModuleInitialized = false
let originalFetch: typeof fetch

function ensureFetchModule(): DenoFetchWasmModule {
  const fetchModule = lazyFetch()
  if (!fetchModuleInitialized) {
    Deno.core.setWasmStreamingCallback(fetchModule.handleWasmStreaming)
    originalFetch = fetchModule.fetch
    fetchModuleInitialized = true
  }

  return fetchModule
}

const requestDedupeMap = new Map<string, Promise<Response>>()
const FILE_EXTENSION_REGEX = /\.([^./]+)$/

interface RequestMeta {
  readonly url: string
  readonly method: string
  readonly headers: HeadersInit | undefined
  readonly cacheMode: RequestCache | undefined
}

type CachedFetchInit = RequestInit & {
  rari?: {
    revalidate?: number | false
    timeout?: number
    tags?: unknown
  }
  next?: {
    revalidate?: number | false
    tags?: unknown
  }
  fetchOptions?: {
    timeout?: number
  }
}

function resolveRequestMeta(input: RequestInfo | URL, init: CachedFetchInit = {}): RequestMeta {
  if (typeof input === 'string') {
    return {
      url: input,
      method: (init.method ?? 'GET').toUpperCase(),
      headers: init.headers,
      cacheMode: init.cache,
    }
  }

  if (input instanceof URL) {
    return {
      url: input.href,
      method: (init.method ?? 'GET').toUpperCase(),
      headers: init.headers,
      cacheMode: init.cache,
    }
  }

  return {
    url: input.url,
    method: (init.method ?? input.method).toUpperCase(),
    headers: init.headers ?? input.headers,
    cacheMode: init.cache ?? input.cache,
  }
}

function extractValidTags(init: CachedFetchInit): string[] {
  const tags = init.rari?.tags ?? init.next?.tags
  if (tags != null && Array.isArray(tags)) {
    // `Array.isArray` narrows to `any[]`; re-widen to `unknown[]` before filtering.
    return (tags as unknown[])
      .filter((tag): tag is string => typeof tag === 'string' && tag.trim().length > 0)
      .map(tag => tag.trim())
  }

  return []
}

function generateCacheKey(input: RequestInfo | URL, init: CachedFetchInit): string {
  const { url, method, headers } = resolveRequestMeta(input, init)
  const headersStr = serializeCacheHeaders(headers != null ? new Headers(headers) : undefined)
  const bodyStr = serializeCacheBody(init.body)
  const tagsStr = serializeCacheTags(init)
  const timeout = init.rari?.timeout ?? 5000
  const revalidate = init.rari?.revalidate ?? init.next?.revalidate
  const optionsStr = `:timeout:${timeout}:revalidate:${revalidate}`
  return `${method}:${url}:${headersStr}:${bodyStr}${tagsStr}${optionsStr}`
}

function serializeCacheHeaders(headers: Headers | undefined): string {
  if (headers == null) return '{}'
  const headerEntries: Array<[string, string]> = []
  for (const [name, value] of headers.entries()) {
    headerEntries.push([name.toLowerCase(), value])
  }
  headerEntries.sort((a, b) => a[0].localeCompare(b[0]))
  return JSON.stringify(headerEntries)
}

function serializeCacheBody(body: BodyInit | null | undefined): string {
  if (body == null) return ''
  const value: any = body
  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
    return String(value)
  }
  if (value instanceof Blob) return `<blob:${value.size}:${value.type}>`
  if (value instanceof ArrayBuffer || ArrayBuffer.isView(value)) {
    return `<buffer:${value.byteLength}>`
  }
  if (value instanceof FormData) return serializeFormDataBody(value)
  if (value instanceof ReadableStream) {
    return `<stream:${Date.now()}:${Math.random().toString(36).slice(2)}>`
  }
  return String(value)
}

function serializeFormDataBody(body: FormData): string {
  const entries = []
  for (const [key, value] of body.entries()) {
    const val: any = value
    if (typeof val === 'string') {
      entries.push(`${key}=${val}`)
    } else if (val instanceof File) {
      entries.push(`${key}=<file:${val.name}:${val.size}:${val.type}>`)
    } else if (val instanceof Blob) {
      entries.push(`${key}=<blob:${val.size}:${val.type}>`)
    }
  }
  return `<formdata:${entries.join('&')}>`
}

function serializeCacheTags(init: CachedFetchInit): string {
  const validTags = extractValidTags(init)
  if (validTags.length === 0) return ''
  validTags.sort((a: string, b: string) => a.localeCompare(b))
  return `:tags:${JSON.stringify(validTags)}`
}

// oxlint-disable typescript/prefer-readonly-parameter-types
function shouldCache(input: RequestInfo | URL, init: CachedFetchInit, meta?: RequestMeta): boolean {
  const { method, cacheMode } = meta ?? resolveRequestMeta(input, init)

  if (cacheMode === 'no-store' || cacheMode === 'no-cache' || cacheMode === 'reload') return false

  const revalidate = init.rari?.revalidate ?? init.next?.revalidate

  if (revalidate === false || revalidate === 0) return false

  if (method !== 'GET') return false

  return true
}

interface TelemetrySpan {
  setAttribute: (key: string, value: string | number | boolean) => unknown
  setStatus: (status: { code: number; message?: string }) => unknown
  end: () => void
}

interface TelemetryInternals {
  readonly TRACING_ENABLED: boolean
  readonly PROPAGATORS: readonly {
    inject: (
      context: unknown,
      carrier: Headers,
      setter: { set: (carrier: Headers, key: string, value: string) => void },
    ) => void
  }[]
  readonly ContextManager: { active: () => unknown }
  builtinTracer: () => {
    startSpan: (
      name: string,
      options: { kind: number; attributes?: Record<string, string | number> },
    ) => TelemetrySpan
  }
  enterSpan: (span: TelemetrySpan) => unknown
  exitSpan: (snapshot: unknown) => void
}

function isTelemetryInternals(value: unknown): value is TelemetryInternals {
  return typeof value === 'object' && value != null && 'builtinTracer' in value
}

const SPAN_KIND_CLIENT = 2
const SPAN_STATUS_ERROR = 2

const REDACTED = 'REDACTED'
const SENSITIVE_QUERY_PARAMS = ['X-Amz-Signature', 'X-Amz-Security-Token']

function sanitizeUrlForSpan(url: string): string {
  let parsed: URL
  try {
    parsed = new URL(url)
  } catch {
    return url
  }
  if (parsed.username !== '' || parsed.password !== '') {
    parsed.username = REDACTED
    parsed.password = REDACTED
  }
  for (const name of SENSITIVE_QUERY_PARAMS) {
    if (parsed.searchParams.has(name)) parsed.searchParams.set(name, REDACTED)
  }
  return parsed.href
}

const SENSITIVE_QUERY_PARAM_PATTERN =
  /\b(X(?:-|%2D)Amz(?:-|%2D)(?:Signature|Security(?:-|%2D)Token))=[^&\s"'()<>,]*/giu
const URL_USERINFO_PATTERN = /\b([a-z][a-z\d+.-]*:\/\/)[^/\s@]*@/giu

function sanitizeErrorMessageForSpan(message: string): string {
  return message
    .replace(SENSITIVE_QUERY_PARAM_PATTERN, `$1=${REDACTED}`)
    .replace(URL_USERINFO_PATTERN, `$1${REDACTED}:${REDACTED}@`)
}

function startFetchSpan(
  url: string,
): { span: TelemetrySpan; traceHeaders: [string, string][] } | null {
  const telemetry = internals.__telemetry
  if (!isTelemetryInternals(telemetry) || !telemetry.TRACING_ENABLED) return null

  let attributes: Record<string, string | number> = {
    'http.request.method': 'GET',
    'url.full': sanitizeUrlForSpan(url),
  }
  try {
    const parsed = new URL(url)
    attributes = {
      ...attributes,
      'server.address': parsed.hostname,
      'url.scheme': parsed.protocol.slice(0, -1),
    }
    if (parsed.port !== '') attributes['server.port'] = Number(parsed.port)
  } catch {}

  const span = telemetry.builtinTracer().startSpan('GET', { kind: SPAN_KIND_CLIENT, attributes })
  const snapshot = telemetry.enterSpan(span)
  const carrier = new Headers()
  try {
    const context = telemetry.ContextManager.active()
    for (const propagator of telemetry.PROPAGATORS) {
      propagator.inject(context, carrier, {
        set(headers, key, value) {
          headers.append(key, value)
        },
      })
    }
  } finally {
    telemetry.exitSpan(snapshot)
  }

  const traceHeaders: [string, string][] = []
  carrier.forEach((value, key) => {
    traceHeaders.push([key, value])
  })
  return { span, traceHeaders }
}

async function fetchWithRustCache(
  input: RequestInfo | URL,
  init: CachedFetchInit,
  meta?: RequestMeta,
): Promise<Response> {
  ensureFetchModule()
  const { url, headers } = meta ?? resolveRequestMeta(input, init)
  const options = buildRustFetchOptions(init, headers)
  const traced = startFetchSpan(url)
  if (traced && traced.traceHeaders.length > 0)
    options.traceHeaders = JSON.stringify(traced.traceHeaders)

  try {
    const rariGlobal = g['~rari']
    const result = await Deno.core.ops.op_fetch_with_cache(
      url,
      JSON.stringify(options),
      rariGlobal?.currentRequestId?.() ?? '',
    )

    if (!result.ok)
      throw new Error(result.error != null && result.error !== '' ? result.error : 'Fetch failed')

    traced?.span.setAttribute('http.response.status_code', result.status)
    traced?.span.setAttribute('rari.fetch.cached', result.cached)
    if (result.status >= 400) traced?.span.setStatus({ code: SPAN_STATUS_ERROR })
    const response = buildCachedFetchResponse(url, result)
    traced?.span.end()
    return response
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error)
    if (traced) {
      traced.span.setStatus({
        code: SPAN_STATUS_ERROR,
        message: sanitizeErrorMessageForSpan(message),
      })
      traced.span.end()
    }
    console.error(
      '[Fetch Cache] Error in fetchWithRustCache, falling back to original fetch:',
      message,
    )
    return originalFetch(input, init)
  }
}

function buildRustFetchOptions(
  init: CachedFetchInit,
  headers: HeadersInit | undefined,
): Record<string, string> {
  const options: Record<string, string> = {}

  if (headers) {
    const normalizedHeaders = new Headers(headers)
    const headerPairs: [string, string][] = []
    normalizedHeaders.forEach((value, key) => {
      headerPairs.push([key, value])
    })
    if (headerPairs.length > 0) options.headers = JSON.stringify(headerPairs)
  }

  const revalidate = init.rari?.revalidate ?? init.next?.revalidate
  if (typeof revalidate === 'number') options.cacheTTLMs = String(revalidate * 1000)

  const validTags = extractValidTags(init)
  if (validTags.length > 0) options.tags = JSON.stringify(validTags)

  const rawTimeout = init.rari?.timeout ?? init.fetchOptions?.timeout
  const timeoutMs = rawTimeout ?? 5000
  options.timeout = String(typeof timeoutMs === 'number' && timeoutMs > 0 ? timeoutMs : 5000)
  return options
}

// https://fetch.spec.whatwg.org/#null-body-status
const NULL_BODY_STATUSES = new Set([101, 103, 204, 205, 304])

function buildCachedFetchResponse(
  url: string,
  result: {
    readonly body?: string
    readonly status: number
    readonly statusText?: string
    readonly headers?: Record<string, unknown>
  },
): Response {
  const responseHeaders = new Headers()
  if (result.headers && typeof result.headers === 'object') {
    for (const [name, value] of Object.entries(result.headers)) {
      if (typeof value === 'string') responseHeaders.set(name, value)
      else responseHeaders.set(name, String(value))
    }
  }

  if (!responseHeaders.has('content-type')) {
    responseHeaders.set('content-type', detectContentType(url, result.body))
  }

  const body = NULL_BODY_STATUSES.has(result.status) ? null : result.body
  return new Response(body, {
    status: result.status,
    statusText:
      result.statusText != null && result.statusText !== ''
        ? result.statusText
        : result.status === 200
          ? 'OK'
          : '',
    headers: responseHeaders,
  })
}

function detectContentType(url: string, body: string | undefined): string {
  const urlPath = url.split('?')[0].split('#')[0]
  const extensionMatch = FILE_EXTENSION_REGEX.exec(urlPath)
  const extension = extensionMatch ? extensionMatch[1].toLowerCase() : null

  if (extension === 'json') return 'application/json'
  if (extension === 'html' || extension === 'htm') return 'text/html'
  if (extension === 'xml') return 'application/xml'
  if (extension === 'txt') return 'text/plain'

  if (body != null && body.length > 0 && body.length < 10000) {
    const trimmed = body.trim()
    if (
      (trimmed.startsWith('{') && trimmed.endsWith('}')) ||
      (trimmed.startsWith('[') && trimmed.endsWith(']'))
    ) {
      return 'application/json'
    }
  }

  return 'text/plain'
}
// oxlint-enable typescript/prefer-readonly-parameter-types

async function cachedFetch(
  input: RequestInfo | URL,
  init: CachedFetchInit = {},
): Promise<Response> {
  ensureFetchModule()
  const meta = resolveRequestMeta(input, init)

  if (!shouldCache(input, init, meta)) return originalFetch(input, init)

  const cacheKey = generateCacheKey(input, init)
  const inFlight = requestDedupeMap.get(cacheKey)

  if (inFlight) {
    const response = await inFlight
    return response.clone()
  }

  const hasRustOp = typeof Deno.core.ops.op_fetch_with_cache === 'function'

  const promise = hasRustOp ? fetchWithRustCache(input, init, meta) : originalFetch(input, init)
  requestDedupeMap.set(cacheKey, promise)
  try {
    return await promise
  } finally {
    requestDedupeMap.delete(cacheKey)
  }
}

applyToGlobal({
  fetch: writeable(cachedFetch),
  Request: propNonEnumerableLazyLoaded(m => m.Request, lazyRequest),
  Response: propNonEnumerableLazyLoaded(m => m.Response, lazyResponse),
  Headers: propNonEnumerableLazyLoaded(m => m.Headers, lazyHeaders),
  FormData: propNonEnumerableLazyLoaded(m => m.FormData, lazyFormData),
  EventSource: propNonEnumerableLazyLoaded(m => m.EventSource, lazyEventSource),
})

Object.defineProperties(g.Deno, {
  HttpClient: {
    get() {
      return lazyHttpClient().HttpClient
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
  createHttpClient: {
    get() {
      return lazyHttpClient().createHttpClient
    },
    set() {},
    enumerable: false,
    configurable: true,
  },
})
