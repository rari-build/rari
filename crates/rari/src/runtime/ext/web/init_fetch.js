/* eslint-disable no-undef */
import * as headers from 'ext:deno_fetch/20_headers.js'
import * as formData from 'ext:deno_fetch/21_formdata.js'
import * as httpClient from 'ext:deno_fetch/22_http_client.js'
import * as request from 'ext:deno_fetch/23_request.js'
import * as response from 'ext:deno_fetch/23_response.js'
import * as fetch from 'ext:deno_fetch/26_fetch.js'
import * as eventSource from 'ext:deno_fetch/27_eventsource.js'
import { applyToGlobal, nonEnumerable, writeable } from 'ext:rari/rari.js'

Deno.core.setWasmStreamingCallback(fetch.handleWasmStreaming)

const originalFetch = fetch.fetch
const requestDedupeMap = new Map()
const FILE_EXTENSION_REGEX = /\.([^./]+)$/

function resolveRequestMeta(input, init = {}) {
  const req = input instanceof Request ? input : null
  return {
    url: typeof input === 'string' ? input : input instanceof URL ? input.href : req.url,
    method: (init.method ?? req?.method ?? 'GET').toUpperCase(),
    headers: init.headers ?? req?.headers,
    cacheMode: init.cache ?? req?.cache,
  }
}

function generateCacheKey(input, init) {
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
    if (typeof init.body === 'string' || typeof init.body === 'number' || typeof init.body === 'boolean') {
      bodyStr = String(init.body)
    }
    else if (init.body instanceof Blob) {
      bodyStr = `<blob:${init.body.size}:${init.body.type}>`
    }
    else if (init.body instanceof ArrayBuffer || ArrayBuffer.isView(init.body)) {
      const size = init.body instanceof ArrayBuffer ? init.body.byteLength : init.body.byteLength
      bodyStr = `<buffer:${size}>`
    }
    else if (init.body instanceof FormData) {
      bodyStr = '<formdata>'
    }
    else if (init.body instanceof ReadableStream) {
      bodyStr = '<stream>'
    }
    else {
      bodyStr = String(init.body)
    }
  }

  return `${method}:${url}:${headersStr}:${bodyStr}`
}

function shouldCache(input, init, meta) {
  const { method, cacheMode } = meta || resolveRequestMeta(input, init)
  if (cacheMode === 'no-store' || cacheMode === 'no-cache' || cacheMode === 'reload') {
    return false
  }
  const revalidate = init?.rari?.revalidate ?? init?.next?.revalidate
  if (revalidate === false || revalidate === 0) {
    return false
  }
  if (method !== 'GET') {
    return false
  }

  return true
}

async function fetchWithRustCache(input, init, meta) {
  const { url, headers } = meta || resolveRequestMeta(input, init)
  const options = {}

  if (headers) {
    const normalizedHeaders = new Headers(headers)
    const headerPairs = []
    normalizedHeaders.forEach((value, key) => {
      headerPairs.push([key, value])
    })
    if (headerPairs.length > 0) {
      options.headers = JSON.stringify(headerPairs)
    }
  }

  const revalidate = init?.rari?.revalidate ?? init?.next?.revalidate
  if (typeof revalidate === 'number') {
    options.cacheTTLMs = String(revalidate * 1000)
  }

  const timeoutMs = init?.rari?.timeout ?? init?.fetchOptions?.timeout ?? 5000
  options.timeout = String(typeof timeoutMs === 'number' && timeoutMs > 0 ? timeoutMs : 5000)

  try {
    const result = await Deno.core.ops.op_fetch_with_cache(url, JSON.stringify(options))

    if (!result.ok) {
      throw new Error(result.error || 'Fetch failed')
    }

    const responseHeaders = new Headers()
    if (result.headers && typeof result.headers === 'object') {
      for (const [name, value] of Object.entries(result.headers)) {
        responseHeaders.set(name, value)
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
  catch (error) {
    console.error('[Fetch Cache] Error in fetchWithRustCache, falling back to original fetch:', error.message)
    return originalFetch(input, init)
  }
}

async function cachedFetch(input, init) {
  const meta = resolveRequestMeta(input, init)

  if (!shouldCache(input, init, meta)) {
    return originalFetch(input, init)
  }

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

Object.defineProperty(cachedFetch, '__rariWrapped', {
  value: true,
  writable: false,
  enumerable: false,
  configurable: false,
})

applyToGlobal({
  fetch: writeable(cachedFetch),
  Request: nonEnumerable(request.Request),
  Response: nonEnumerable(response.Response),
  Headers: nonEnumerable(headers.Headers),
  FormData: nonEnumerable(formData.FormData),
  EventSource: nonEnumerable(eventSource.EventSource),
})

globalThis.Deno.HttpClient = httpClient.HttpClient
globalThis.Deno.createHttpClient = httpClient.createHttpClient
globalThis.__rariFetchCacheInstalled = true
