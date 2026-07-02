/// <reference path="../../types.d.ts" />

interface ClearResult {
  success: boolean
  clearedCount?: number
  componentId: string
  error?: string
}

(function (): ClearResult {
  const NON_ALPHANUMERIC_REGEX = /[^a-z0-9]/gi
  try {
    const componentId = '{component_id}'
    let clearedCount = 0

    if (g[componentId]) {
      delete g[componentId]
      clearedCount++
    }

    const registrationKey = `Component_${componentId.replace(NON_ALPHANUMERIC_REGEX, '_')}`
    if (g[registrationKey]) {
      delete g[registrationKey]
      clearedCount++
    }

    if (g['~rsc']?.modules && g['~rsc'].modules[componentId]) {
      delete g['~rsc'].modules[componentId]
      clearedCount++
    }

    if (g['~rsc']?.functions && g['~rsc'].functions[componentId]) {
      delete g['~rsc'].functions[componentId]
      clearedCount++
    }

    return {
      success: true,
      clearedCount,
      componentId,
    }
  }
  catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error)
    return {
      success: false,
      error: errorMessage,
      componentId: '{component_id}',
    }
  }
})()
