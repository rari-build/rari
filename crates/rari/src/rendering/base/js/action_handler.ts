/// <reference path="../../types.d.ts" />

interface FlightServerActions {
  decodeAction: (
    body: FormData,
    serverManifest: Record<string, { id: string, name?: string, chunks: string[] }>,
  ) => Promise<(() => Promise<unknown>) | null>
  decodeFormState: (
    actionResult: unknown,
    body: FormData,
    serverManifest: Record<string, { id: string, name?: string, chunks: string[] }>,
  ) => Promise<unknown>
  decodeReply: (
    body: string | FormData,
    serverManifest: Record<string, { id: string, name?: string, chunks: string[] }>,
  ) => Promise<unknown>
}

(async () => {
  try {
    if (typeof g['~rari']?.loadRscReactVendors === 'function')
      g['~rari'].loadRscReactVendors()

    const flightServer = g['~reactServerRenderer'] as FlightServerActions | undefined
    if (!flightServer?.decodeAction || !flightServer?.decodeReply || !flightServer?.decodeFormState)
      throw new TypeError('Flight server action helpers not loaded')

    const serverManifest = g['~rari']?.serverManifest || {}
    const mode = __RARI_ACTION_MODE__
    const actionId = __RARI_ACTION_ID__
    const bodyText = __RARI_ACTION_BODY__
    const bodyBase64 = __RARI_ACTION_BODY_B64__
    const contentType = __RARI_ACTION_CONTENT_TYPE__
    const formEntries = __RARI_ACTION_FORM_ENTRIES__

    if (mode === 'form') {
      let formData: FormData
      if (contentType && bodyBase64) {
        const binary = Uint8Array.from(atob(bodyBase64), char => char.charCodeAt(0))
        const request = new Request('http://rari.invalid', {
          method: 'POST',
          headers: { 'Content-Type': contentType },
          body: binary,
        })
        formData = await request.formData()
      }
      else {
        formData = new FormData()
        for (const [key, value] of formEntries)
          formData.append(key, value)
      }

      validateFormData(formData)

      const runFormAction = await flightServer.decodeAction(formData, serverManifest)
      if (!runFormAction)
        throw new TypeError('Failed to decode server action from form data')

      const actionResult = await runFormAction()
      const formState = await flightServer.decodeFormState(actionResult, formData, serverManifest)

      if (formState != null) {
        if (!g['~rari'])
          g['~rari'] = {}
        g['~rari'].actionFormState = formState

        if (actionResult && typeof actionResult === 'object') {
          return {
            ...(actionResult as Record<string, unknown>),
            '~rariFormState': formState,
          }
        }

        return {
          'value': actionResult,
          '~rariFormState': formState,
        }
      }

      return actionResult
    }

    let decoded: unknown
    if (mode === 'reply-multipart') {
      const binary = Uint8Array.from(atob(bodyBase64), char => char.charCodeAt(0))
      const request = new Request('http://rari.invalid', {
        method: 'POST',
        headers: { 'Content-Type': contentType },
        body: binary,
      })
      const formData = await request.formData()
      decoded = await flightServer.decodeReply(formData, serverManifest)
    }
    else {
      decoded = await flightServer.decodeReply(bodyText, serverManifest)
    }

    const args = Array.isArray(decoded) ? decoded : [decoded]
    const sanitizedArgs = validateActionArgs(args)
    const actionFn = resolveActionFn(actionId, serverManifest)
    const result = await actionFn(...sanitizedArgs)
    return stashRpcActionResult(result)
  }
  catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error)
    throw new Error(`Server action error: ${errorMessage}`)
  }
})()
