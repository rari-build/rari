/// <reference path="../../types.d.ts" />

interface ClearResult {
  success: boolean
  clearedCount?: number
  componentId: string
  error?: string
}

;(function (): ClearResult {
  const NON_ALPHANUMERIC_REGEX = /[^a-z0-9]/gi
  try {
    const componentId = '{component_id}'
    let clearedCount = 0

    if (g[componentId] != null) {
      delete g[componentId]
      clearedCount++
    }

    const registrationKey = `Component_${componentId.replace(NON_ALPHANUMERIC_REGEX, '_')}`
    if (g[registrationKey] != null) {
      delete g[registrationKey]
      clearedCount++
    }

    if (g['~rsc']?.modules?.[componentId] != null) {
      delete g['~rsc'].modules[componentId]
      clearedCount++
    }

    if (g['~rsc']?.functions?.[componentId] != null) {
      delete g['~rsc'].functions[componentId]
      clearedCount++
    }

    return {
      success: true,
      clearedCount,
      componentId,
    }
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error)
    return {
      success: false,
      error: errorMessage,
      componentId: '{component_id}',
    }
  }
})()
