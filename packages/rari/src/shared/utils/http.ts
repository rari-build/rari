export async function throwIfNotOk(response: Response): Promise<void> {
  if (!response.ok) {
    let errorText: string
    try {
      errorText = await response.text()
    }
    catch {
      errorText = '<unable to read response body>'
    }
    const MAX_PREVIEW_LENGTH = 200
    const bodyPreview = errorText.length > MAX_PREVIEW_LENGTH
      ? `${errorText.slice(0, MAX_PREVIEW_LENGTH)}...`
      : errorText
    const error = new Error(`HTTP ${response.status}: ${response.statusText}`) as Error & { status: number, statusText: string, bodyPreview: string }
    error.status = response.status
    error.statusText = response.statusText
    error.bodyPreview = bodyPreview
    throw error
  }
}

export async function assertResponseOk(response: Response): Promise<Response> {
  await throwIfNotOk(response)
  return response
}
