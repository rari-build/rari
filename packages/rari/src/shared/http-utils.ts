export async function throwIfNotOk(response: Response): Promise<void> {
  if (!response.ok) {
    let errorText: string
    try {
      errorText = await response.text()
    }
    catch {
      errorText = '<unable to read response body>'
    }
    const error = new Error(`HTTP ${response.status}: ${errorText}`) as Error & { status: number, statusText: string }
    error.status = response.status
    error.statusText = response.statusText
    throw error
  }
}

export async function assertResponseOk(response: Response): Promise<Response> {
  await throwIfNotOk(response)
  return response
}
