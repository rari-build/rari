/// <reference path="../../types.d.ts" />

interface FlightServerActions {
  decodeAction: (
    body: FormData,
    serverManifest: Readonly<{
      readonly [key: string]: Readonly<{
        readonly id: string
        readonly name?: string
        readonly chunks: readonly string[]
      }>
    }>,
  ) => Promise<(() => Promise<unknown>) | null>
  decodeFormState: (
    actionResult: unknown,
    body: FormData,
    serverManifest: Readonly<{
      readonly [key: string]: Readonly<{
        readonly id: string
        readonly name?: string
        readonly chunks: readonly string[]
      }>
    }>,
  ) => Promise<unknown>
  decodeReply: (
    body: string | FormData,
    serverManifest: Readonly<{
      readonly [key: string]: Readonly<{
        readonly id: string
        readonly name?: string
        readonly chunks: readonly string[]
      }>
    }>,
  ) => Promise<unknown>
}

void (async () => {
  try {
    if (typeof g['~rari']?.loadRscReactVendors === 'function') g['~rari'].loadRscReactVendors()

    const flightServer = g['~reactServerRenderer'] as FlightServerActions | undefined // oxlint-disable-line typescript/no-unsafe-type-assertion -- flight runtime global
    if (
      flightServer == null ||
      typeof flightServer.decodeAction !== 'function' ||
      typeof flightServer.decodeReply !== 'function' ||
      typeof flightServer.decodeFormState !== 'function'
    ) {
      throw new TypeError('Flight server action helpers not loaded')
    }

    type ServerManifest = Record<string, { id: string; name?: string; chunks: string[] }>
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- runtime manifest may be partial before routes register
    const serverManifest = (g['~rari']?.serverManifest ?? {}) as ServerManifest
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
      } else {
        formData = new FormData()
        for (const [key, value] of formEntries) formData.append(key, value)
      }

      validateFormData(formData)

      const runFormAction = await flightServer.decodeAction(formData, serverManifest)
      if (!runFormAction) throw new TypeError('Failed to decode server action from form data')

      const actionResult = await runFormAction()
      const formState = await flightServer.decodeFormState(actionResult, formData, serverManifest)

      if (formState != null) {
        g['~rari'] ??= {}
        g['~rari'].actionFormState = formState

        if (
          actionResult != null &&
          typeof actionResult === 'object' &&
          !Array.isArray(actionResult)
        ) {
          return {
            ...Object.fromEntries(Object.entries(actionResult)),
            '~rariFormState': formState,
            '~rariSkipRefresh': true,
          }
        }

        return {
          'value': actionResult,
          '~rariFormState': formState,
          '~rariSkipRefresh': true,
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
    } else {
      decoded = await flightServer.decodeReply(bodyText, serverManifest)
    }

    const args = Array.isArray(decoded) ? decoded : [decoded]
    const sanitizedArgs = validateActionArgs(args)
    const actionFn = resolveActionFn(actionId, serverManifest)
    const result = await actionFn(...sanitizedArgs)
    return stashRpcActionResult(result)
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error)
    throw new Error(`Server action error: ${errorMessage}`)
  }
})()
