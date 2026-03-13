export default {
  async fetch(request, env, ctx) {
    if (request.method !== 'GET')
      return fetch(request)

    const url = new URL(request.url)

    if (url.pathname.startsWith('/assets/') || url.pathname.startsWith('/_rari/')) {
      return cacheFirst(request, ctx, 60 * 60 * 24 * 365)
    }

    return edgeSWR(request, ctx, false)
  },
}

async function cacheFirst(request, ctx, ttl) {
  const cache = caches.default
  const cached = await cache.match(request)
  if (cached)
    return cached

  const response = await fetch(request)

  if (response.ok) {
    const toCache = response.clone()
    const headers = new Headers(toCache.headers)
    headers.set('Cache-Control', `public, max-age=${ttl}, immutable`)
    const cachedResponse = new Response(toCache.body, {
      status: toCache.status,
      statusText: toCache.statusText,
      headers,
    })
    ctx.waitUntil(cache.put(request, cachedResponse))
    return response
  }

  return response
}

async function edgeSWR(originRequest, ctx, skipStale = false) {
  const cache = caches.default
  const accept = originRequest.headers.get('Accept') || ''
  const isRSC = accept.includes('text/x-component')

  const cached = await cache.match(originRequest)

  let validCached = null
  if (cached) {
    const cachedContentType = cached.headers.get('content-type') || ''
    const isRSCResponse = cachedContentType.includes('text/x-component')

    if (isRSC === isRSCResponse && cached.ok) {
      validCached = cached
      if (!skipStale) {
        ctx.waitUntil(revalidate(originRequest, cached, cache))
        return cached
      }
    }
  }

  const headers = new Headers(originRequest.headers)
  if (validCached && !skipStale) {
    const etag = validCached.headers.get('ETag')
    if (etag)
      headers.set('If-None-Match', etag)
  }

  const response = await fetch(new Request(originRequest.url, {
    method: 'GET',
    headers,
  }))

  if (response.status === 304 && validCached) {
    return validCached
  }

  if (response.ok && isCacheable(response)) {
    const toCache = response.clone()
    ctx.waitUntil(cache.put(originRequest, toCache))
    return response
  }

  if (!response.ok && validCached) {
    const staleResponse = new Response(validCached.body, {
      status: validCached.status,
      statusText: validCached.statusText,
      headers: new Headers(validCached.headers),
    })
    staleResponse.headers.set('X-Served-Stale', 'true')
    staleResponse.headers.set('X-Origin-Status', response.status.toString())
    return staleResponse
  }

  if (!response.ok) {
    const headers = new Headers(response.headers)
    headers.set('X-No-Cache-Available', 'true')
    return new Response(response.body, {
      status: response.status,
      statusText: response.statusText,
      headers,
    })
  }

  return response
}

async function revalidate(originRequest, cached, cache) {
  try {
    const headers = new Headers(originRequest.headers)
    const etag = cached.headers.get('ETag')
    if (etag)
      headers.set('If-None-Match', etag)

    const response = await fetch(new Request(originRequest.url, {
      method: 'GET',
      headers,
    }))

    if (response.status === 304)
      return

    if (response.ok && isCacheable(response)) {
      const toCache = response.clone()
      await cache.put(originRequest, toCache)
    }
  }
  catch {
    // Silently fail revalidation
  }
}

function isCacheable(response) {
  const cc = response.headers.get('Cache-Control') || ''
  return !cc.includes('no-store') && !cc.includes('no-cache')
}
