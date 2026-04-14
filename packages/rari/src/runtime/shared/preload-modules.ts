import { getClientComponentAsync } from './get-client-component'

export async function preloadModulesFromWireFormat(wireFormat: string): Promise<void> {
  const lines = wireFormat.split('\n')
  const moduleIds: string[] = []

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
          moduleIds.push(importData.id)
        }
      }
      catch {}
    }
  }

  if (moduleIds.length > 0) {
    await Promise.all(moduleIds.map(async (id) => {
      try {
        const component = await getClientComponentAsync(id)
        if (!component)
          console.warn(`[rari] Failed to preload component: ${id}`)
      }
      catch (error) {
        console.error(`[rari] Error preloading component ${id}:`, error)
      }
    }))
  }
}
