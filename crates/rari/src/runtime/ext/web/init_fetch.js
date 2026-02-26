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

function generateCacheKey(input, init) {
  const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
  const method = init?.method || 'GET'

  let headersStr = '{}'
  if (init?.headers) {
    const headerEntries = []
    const headers = init.headers

    if (typeof headers.entries === 'function') {
      for (const [name, value] of headers.entries()) {
        headerEntries.push([name.toLowerCase(), value])
      }
    }
    else if (typeof headers.forEach === 'function') {
      headers.forEach((value, name) => {
        headerEntries.push([name.toLowerCase(), value])
      })
    }
    else if (typeof headers === 'object') {
      for (const [name, value] of Object.entries(headers)) {
        headerEntries.push([name.toLowerCase(), String(value)])
      }
    }

    headerEntries.sort((a, b) => a[0].localeCompare(b[0]))
    headersStr = JSON.stringify(headerEntries)
  }

  const body = init?.body ? String(init.body) : ''
  return `${method}:${url}:${headersStr}:${body}`
}

function shouldCache(init) {
  if (init?.cache === 'no-store' || init?.cache === 'no-cache') {
    return false
  }
  const revalidate = init?.rari?.revalidate ?? init?.next?.revalidate
  if (revalidate === false || revalidate === 0) {
    return false
  }
  const method = init?.method?.toUpperCase() || 'GET'
  if (method !== 'GET' && method !== 'HEAD') {
    return false
  }

  return true
}

async function fetchWithRustCache(input, init) {
  const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
  const options = {}

  if (init?.headers) {
    const headers = new Headers(init.headers)
    const headerPairs = []
    headers.forEach((value, key) => {
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

  options.timeout = '5000'

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
      statusText: result.statusText || 'OK',
      headers: responseHeaders,
    })
  }
  catch (error) {
    console.error('[Fetch Cache] Error in fetchWithRustCache, falling back to original fetch:', error.message)
    return originalFetch(input, init)
  }
}

async function cachedFetch(input, init) {
  if (!shouldCache(init)) {
    return originalFetch(input, init)
  }

  const cacheKey = generateCacheKey(input, init)
  const inFlight = requestDedupeMap.get(cacheKey)

  if (inFlight) {
    const response = await inFlight
    return response.clone()
  }

  const hasRustOp = typeof Deno?.core?.ops?.op_fetch_with_cache === 'function'

  if (hasRustOp) {
    const promise = fetchWithRustCache(input, init)
    requestDedupeMap.set(cacheKey, promise)
    try {
      const response = await promise
      return response.clone()
    }
    finally {
      requestDedupeMap.delete(cacheKey)
    }
  }
  else {
    const promise = originalFetch(input, init)
    requestDedupeMap.set(cacheKey, promise)
    try {
      const response = await promise
      return response.clone()
    }
    finally {
      requestDedupeMap.delete(cacheKey)
    }
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
