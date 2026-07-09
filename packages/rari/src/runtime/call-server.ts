import { encodeReply } from 'virtual:react-flight-client'

export interface ServerActionResult {
  success: boolean
  result?: unknown
  error?: string
  redirect?: string
}

function applyRedirect(redirect: string) {
  if (typeof window !== 'undefined') {
    const absoluteRedirect = new URL(redirect, window.location.href).href
    if (absoluteRedirect !== window.location.href)
      window.location.href = absoluteRedirect
  }
}

export async function callServer(id: string, args: unknown[]): Promise<unknown> {
  const encoded = await encodeReply(args)
  const headers: Record<string, string> = {
    'Accept': 'application/json',
    'rsc-action-id': id,
  }

  let body: BodyInit
  if (typeof encoded === 'string') {
    headers['Content-Type'] = 'text/plain;charset=UTF-8'
    body = encoded
  }
  else {
    body = encoded
  }

  const response = await fetch('/_rari/action', {
    method: 'POST',
    headers,
    body,
  })

  if (!response.ok) {
    const errorText = await response.text().catch(() => response.statusText)
    throw new Error(`Server action "${id}" failed with status ${response.status}: ${errorText}`)
  }

  const payload = await response.json() as ServerActionResult

  if (payload.redirect) {
    applyRedirect(payload.redirect)
    return { redirect: payload.redirect }
  }

  if (!payload.success)
    throw new Error(payload.error || 'Server action failed')

  return payload.result
}
