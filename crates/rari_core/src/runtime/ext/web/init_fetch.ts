/// <reference path="../types.d.ts" />

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
const lazyEventSource = lazyExtScript<DenoFetchEventSourceModule>('ext:deno_fetch/27_eventsource.js')

let fetchModuleInitialized = false
let originalFetch: typeof fetch

function ensureFetchModule(): DenoFetchWasmModule {
  const fetchModule = lazyFetch()
  if (!fetchModuleInitialized) {
    Deno.core.setWasmStreamingCallback(fetchModule.handleWasmStreaming)
    originalFetch = fetchModule.fetch as typeof fetch
    fetchModuleInitialized = true
  }

  return fetchModule
}

const requestDedupeMap = new Map<string, Promise<Response>>()
const FILE_EXTENSION_REGEX = /\.([^./]+)$/

interface RequestMeta {
  url: string
  method: string
  headers: HeadersInit | undefined
  cacheMode: RequestCache | undefined
}

function resolveRequestMeta(input: RequestInfo | URL, init: RequestInit = {}): RequestMeta {
  const req = input instanceof Request ? input : null
  return {
    url: typeof input === 'string' ? input : input instanceof URL ? input.href : req?.url ?? '',
    method: (init.method ?? req?.method ?? 'GET').toUpperCase(),
    headers: init.headers ?? req?.headers,
    cacheMode: init.cache ?? req?.cache,
  }
}

function extractValidTags(init: RequestInit & { rari?: { tags?: unknown }, next?: { tags?: unknown } }) {
  const tags = init?.rari?.tags ?? init?.next?.tags
  if (tags && Array.isArray(tags)) {
    return tags
      .filter(tag => typeof tag === 'string' && tag.trim().length > 0)
      .map(tag => tag.trim())
  }

  return []
}

function generateCacheKey(input: RequestInfo | URL, init: RequestInit & { rari?: { timeout?: number, revalidate?: number | false, tags?: unknown }, next?: { revalidate?: number | false, tags?: unknown } }): string {
  const { url, method, headers } = resolveRequestMeta(input, init)

  let headersStr = '{}'
  if (headers) {
    const headerEntries = []
    const normalizedHeaders = new Headers(headers)
    for (const [name, value] of normalizedHeaders.entries()) {
      headerEntries.push([name.toLowerCase(), value])
    }

    headerEntries.sort((a, b) => a[0].localeCompare(b[0]))
    headersStr = JSON.stringify(headerEntries)
  }

  let bodyStr = ''
  if (init?.body) {
    const body: any = init.body
    if (typeof body === 'string' || typeof body === 'number' || typeof body === 'boolean') {
      bodyStr = String(body)
    }
    else if (body instanceof Blob) {
      bodyStr = `<blob:${body.size}:${body.type}>`
    }
    else if (body instanceof ArrayBuffer || ArrayBuffer.isView(body)) {
      const size = body instanceof ArrayBuffer ? body.byteLength : body.byteLength
      bodyStr = `<buffer:${size}>`
    }
    else if (body instanceof FormData) {
      const entries = []
      for (const [key, value] of body.entries()) {
        const val: any = value
        if (typeof val === 'string') {
          entries.push(`${key}=${val}`)
        }
        else if (val instanceof File) {
          entries.push(`${key}=<file:${val.name}:${val.size}:${val.type}>`)
        }
        else if (val instanceof Blob) {
          entries.push(`${key}=<blob:${val.size}:${val.type}>`)
        }
      }
      bodyStr = `<formdata:${entries.join('&')}>`
    }
    else if (body instanceof ReadableStream) {
      bodyStr = `<stream:${Date.now()}:${Math.random().toString(36).slice(2)}>`
    }
    else {
      bodyStr = String(body)
    }
  }

  const validTags = extractValidTags(init)
  let tagsStr = ''
  if (validTags.length > 0) {
    const normalizedTags = [...validTags].sort((a: string, b: string) => a.localeCompare(b))
    tagsStr = `:tags:${JSON.stringify(normalizedTags)}`
  }

  const timeout = init?.rari?.timeout ?? 5000
  const revalidate = init?.rari?.revalidate ?? init?.next?.revalidate
  const optionsStr = `:timeout:${timeout}:revalidate:${revalidate}`

  return `${method}:${url}:${headersStr}:${bodyStr}${tagsStr}${optionsStr}`
}

function shouldCache(input: RequestInfo | URL, init: RequestInit & { rari?: { revalidate?: number | false }, next?: { revalidate?: number | false } }, meta?: RequestMeta): boolean {
  const { method, cacheMode } = meta || resolveRequestMeta(input, init)

  if (cacheMode === 'no-store' || cacheMode === 'no-cache' || cacheMode === 'reload')
    return false

  const revalidate = init?.rari?.revalidate ?? init?.next?.revalidate

  if (revalidate === false || revalidate === 0)
    return false

  if (method !== 'GET')
    return false

  return true
}

async function fetchWithRustCache(input: RequestInfo | URL, init: RequestInit & { rari?: { revalidate?: number, timeout?: number, tags?: unknown }, next?: { revalidate?: number, tags?: unknown }, fetchOptions?: { timeout?: number } }, meta?: RequestMeta): Promise<Response> {
  ensureFetchModule()
  const { url, headers } = meta || resolveRequestMeta(input, init)
  const options: Record<string, string> = {}

  if (headers) {
    const normalizedHeaders = new Headers(headers)
    const headerPairs: [string, string][] = []
    normalizedHeaders.forEach((value, key) => {
      headerPairs.push([key, value])
    })
    if (headerPairs.length > 0)
      options.headers = JSON.stringify(headerPairs)
  }

  const revalidate = init?.rari?.revalidate ?? init?.next?.revalidate
  if (typeof revalidate === 'number')
    options.cacheTTLMs = String(revalidate * 1000)

  const validTags = extractValidTags(init)
  if (validTags.length > 0)
    options.tags = JSON.stringify(validTags)

  const timeoutMs = (init?.rari?.timeout ?? init?.fetchOptions?.timeout) || 5000
  options.timeout = String(typeof timeoutMs === 'number' && timeoutMs > 0 ? timeoutMs : 5000)

  try {
    const result = await Deno.core.ops.op_fetch_with_cache(url, JSON.stringify(options))

    if (!result.ok)
      throw new Error(result.error || 'Fetch failed')

    const responseHeaders = new Headers()
    if (result.headers && typeof result.headers === 'object') {
      for (const [name, value] of Object.entries(result.headers)) {
        responseHeaders.set(name, String(value))
      }
    }

    if (!responseHeaders.has('content-type')) {
      let detectedType = 'text/plain'
      const urlPath = url.split('?')[0].split('#')[0]
      const extensionMatch = urlPath.match(FILE_EXTENSION_REGEX)
      const extension = extensionMatch ? extensionMatch[1].toLowerCase() : null

      if (extension === 'json') {
        detectedType = 'application/json'
      }
      else if (extension === 'html' || extension === 'htm') {
        detectedType = 'text/html'
      }
      else if (extension === 'xml') {
        detectedType = 'application/xml'
      }
      else if (extension === 'txt') {
        detectedType = 'text/plain'
      }
      else if (result.body && result.body.length > 0 && result.body.length < 10000) {
        const trimmed = result.body.trim()
        if ((trimmed.startsWith('{') && trimmed.endsWith('}'))
          || (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
          detectedType = 'application/json'
        }
      }

      responseHeaders.set('content-type', detectedType)
    }

    return new Response(result.body, {
      status: result.status,
      statusText: result.statusText || (result.status === 200 ? 'OK' : ''),
      headers: responseHeaders,
    })
  }
  catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error)
    console.error('[Fetch Cache] Error in fetchWithRustCache, falling back to original fetch:', message)
    return originalFetch(input, init)
  }
}

async function cachedFetch(input: RequestInfo | URL, init: RequestInit = {}): Promise<Response> {
  ensureFetchModule()
  const meta = resolveRequestMeta(input, init)

  if (!shouldCache(input, init, meta))
    return originalFetch(input, init)

  const cacheKey = generateCacheKey(input, init)
  const inFlight = requestDedupeMap.get(cacheKey)

  if (inFlight) {
    const response = await inFlight
    return response.clone()
  }

  const hasRustOp = typeof Deno?.core?.ops?.op_fetch_with_cache === 'function'

  const promise = hasRustOp ? fetchWithRustCache(input, init, meta) : originalFetch(input, init)
  requestDedupeMap.set(cacheKey, promise)
  try {
    return await promise
  }
  finally {
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
