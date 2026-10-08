import { toPosixPath } from '@/shared/utils/path'
import { isFlightImportTuple } from '@/shared/utils/type-guards'
import { getClientComponent } from './get-client-component'

export async function preloadModulesFromFlightProtocol(
  flightProtocol: string,
  preloadedModuleIds?: Set<string>,
): Promise<void> {
  const moduleIds = collectFlightModuleIds(flightProtocol, preloadedModuleIds)
  if (moduleIds.size === 0) return

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

function collectFlightModuleIds(
  flightProtocol: string,
  preloadedModuleIds?: ReadonlySet<string>,
): Set<string> {
  const moduleIds = new Set<string>()
  for (const line of flightProtocol.split('\n')) {
    const moduleId = parseFlightImportModuleId(line)
    if (moduleId == null) continue
    if (!preloadedModuleIds || !preloadedModuleIds.has(moduleId)) moduleIds.add(moduleId)
  }
  return moduleIds
}

function parseFlightImportModuleId(line: string): string | null {
  const trimmed = line.trim()
  if (!trimmed) return null

  const colonIndex = trimmed.indexOf(':')
  if (colonIndex === -1) return null

  const content = trimmed.substring(colonIndex + 1)
  if (!content.startsWith('I')) return null

  try {
    const importData: unknown = JSON.parse(content.substring(1))
    if (!isFlightImportTuple(importData)) return null

    const id = importData[0]
    if (!id.includes('/') && id.startsWith('$')) return null
    const exportName = typeof importData[2] === 'string' ? importData[2] : undefined
    const normalizedImportId = toPosixPath(id)

    if (normalizedImportId.includes('#')) return normalizedImportId
    if (exportName != null && exportName !== '' && exportName !== 'default') {
      return `${normalizedImportId}#${exportName}`
    }
    return normalizedImportId
  } catch {
    return null
  }
}
