export class HttpError extends Error {
  readonly status: number
  readonly statusText: string
  readonly bodyPreview: string

  constructor(status: number, statusText: string, bodyPreview: string) {
    super(`HTTP ${status}: ${statusText}`)
    this.name = 'HttpError'
    this.status = status
    this.statusText = statusText
    this.bodyPreview = bodyPreview
  }
}

export async function throwIfNotOk(response: Response): Promise<void> {
  if (!response.ok) {
    let errorText: string
    try {
      errorText = await response.text()
    } catch {
      errorText = '<unable to read response body>'
    }
    const MAX_PREVIEW_LENGTH = 200
    const bodyPreview =
      errorText.length > MAX_PREVIEW_LENGTH
        ? `${errorText.slice(0, MAX_PREVIEW_LENGTH)}...`
        : errorText
    throw new HttpError(response.status, response.statusText, bodyPreview)
  }
}

export async function assertResponseOk(response: Response): Promise<Response> {
  await throwIfNotOk(response)
  return response
}
