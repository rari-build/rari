export async function throwIfNotOk(response: Response): Promise<void> {
  if (!response.ok) {
    const errorText = await response.text()
    throw new Error(`HTTP ${response.status}: ${errorText}`)
  }
}

export async function assertResponseOk(response: Response): Promise<Response> {
  await throwIfNotOk(response)
  return response
}
