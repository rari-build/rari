import { getClientComponentAsync } from './get-client-component'

export async function preloadModulesFromWireFormat(
  wireFormat: string,
  preloadedModuleIds?: Set<string>,
): Promise<void> {
  const lines = wireFormat.split('\n')
  const moduleIds = new Set<string>()

  for (const line of lines) {
    const trimmed = line.trim()
    if (!trimmed)
      continue

    const colonIndex = trimmed.indexOf(':')
    if (colonIndex === -1)
      continue

    const content = trimmed.substring(colonIndex + 1)

    if (content.startsWith('I{')) {
      try {
        const jsonContent = content.substring(1)
        const importData = JSON.parse(jsonContent)

        if (typeof importData === 'object' && !Array.isArray(importData) && importData.id) {
          const normalizedImportId = importData.id.replace(/\\/g, '/')
          const moduleId = importData.name && importData.name !== 'default'
            ? `${normalizedImportId}#${importData.name}`
            : normalizedImportId

          if (!preloadedModuleIds || !preloadedModuleIds.has(moduleId))
            moduleIds.add(moduleId)
        }
      }
      catch {}
    }
  }

  if (moduleIds.size > 0) {
    await Promise.all(Array.from(moduleIds, async (id) => {
      try {
        const component = await getClientComponentAsync(id)

        if (!component) {
          console.warn(`[rari] Failed to preload component: ${id}`)
          return
        }

        preloadedModuleIds?.add(id)
      }
      catch (error) {
        console.error(`[rari] Error preloading component ${id}:`, error)
      }
    }))
  }
}
