import { isFlightImportTuple } from '@/shared/utils/type-guards'
import { getClientComponent } from './get-client-component'

export async function preloadModulesFromFlightProtocol(
  flightProtocol: string,
  preloadedModuleIds?: Set<string>,
): Promise<void> {
  const lines = flightProtocol.split('\n')
  const moduleIds = new Set<string>()

  for (const line of lines) {
    const trimmed = line.trim()
    if (!trimmed) continue

    const colonIndex = trimmed.indexOf(':')
    if (colonIndex === -1) continue

    const content = trimmed.substring(colonIndex + 1)

    if (content.startsWith('I')) {
      try {
        const jsonContent = content.substring(1)
        const importData: unknown = JSON.parse(jsonContent)

        if (isFlightImportTuple(importData)) {
          const id = importData[0]
          const exportName = typeof importData[2] === 'string' ? importData[2] : undefined
          const normalizedImportId = id.replace(/\\/g, '/')

          let moduleId: string
          if (normalizedImportId.includes('#')) {
            moduleId = normalizedImportId
          } else {
            moduleId =
              exportName != null && exportName !== '' && exportName !== 'default'
                ? `${normalizedImportId}#${exportName}`
                : normalizedImportId
          }

          if (!preloadedModuleIds || !preloadedModuleIds.has(moduleId)) moduleIds.add(moduleId)
        }
      } catch {}
    }
  }

  if (moduleIds.size > 0) {
    await Promise.all(
      Array.from(moduleIds, async id => {
        try {
          const component: unknown = await getClientComponent(id)

          if (component == null) {
            console.warn(`[rari] Failed to preload component: ${id}`)
            return
          }

          preloadedModuleIds?.add(id)
        } catch (error) {
          console.error(`[rari] Error preloading component ${id}:`, error)
        }
      }),
    )
  }
}
