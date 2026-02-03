export function getCsrfToken(): string | null {
  if (typeof window === 'undefined')
    return null
  const meta = document.querySelector('meta[name="csrf-token"]')
  return meta ? meta.getAttribute('content') : null
}

export async function refreshCsrfToken(): Promise<boolean> {
  if (typeof window === 'undefined')
    return false

  try {
    const response = await fetch('/_rari/csrf-token')
    if (!response.ok) {
      console.error('[rari] CSRF: Failed to refresh CSRF token:', response.status)
      return false
    }
    const data = await response.json()
    if (data.token) {
      let meta = document.querySelector('meta[name="csrf-token"]')
      if (!meta) {
        meta = document.createElement('meta')
        meta.setAttribute('name', 'csrf-token')
        document.head.appendChild(meta)
      }
      meta.setAttribute('content', data.token)
      return true
    }

    return false
  }
  catch (error) {
    console.error('[rari] CSRF: Error refreshing CSRF token:', error)
    return false
  }
}

export async function fetchWithCsrf(
  url: string,
  options: RequestInit = {},
): Promise<Response> {
  let token = getCsrfToken()

  if (!token) {
    await refreshCsrfToken()
    token = getCsrfToken()
  }

  const headers = new Headers(options.headers)
  if (token)
    headers.set('X-CSRF-Token', token)

  const request = new Request(url, { ...options, headers })
  let retryRequest: Request | null = null
  try {
    retryRequest = request.clone()
  }
  catch {
    retryRequest = null
  }

  const response = await fetch(request)

  /* v8 ignore start - retry logic with token refresh */
  if (response.status === 403 && url.includes('/_rari/')) {
    const refreshed = await refreshCsrfToken()
    if (refreshed) {
      const retryToken = getCsrfToken()
      if (retryToken && retryRequest) {
        headers.set('X-CSRF-Token', retryToken)
        return fetch(new Request(retryRequest, { headers }))
      }
    }
  }
  /* v8 ignore stop */

  return response
}

/* v8 ignore start - browser initialization code */
if (typeof window !== 'undefined') {
  ;(window as any).getCsrfToken = getCsrfToken
  ;(window as any).fetchWithCsrf = fetchWithCsrf
  ;(window as any).refreshCsrfToken = refreshCsrfToken

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
      refreshCsrfToken()
    })
  }
  else {
    refreshCsrfToken()
  }
}
/* v8 ignore stop */
