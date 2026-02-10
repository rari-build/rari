import { ModuleData } from './types'

export async function preloadComponentsFromModules(modules: Map<string, ModuleData>): Promise<void> {
  const clientComponents = (globalThis as any)['~clientComponents'] || {}
  const loadPromises: Promise<any>[] = []

  for (const [, moduleData] of modules) {
    const lookupKeys = [
      moduleData.id,
      `${moduleData.id}#${moduleData.name || 'default'}`,
      moduleData.id.replace(/^src\//, ''),
      moduleData.id.replace(/\\/g, '/'),
    ]

    for (const key of lookupKeys) {
      const componentInfo = clientComponents[key]
      if (componentInfo?.loader && !componentInfo.component && !componentInfo.loading) {
        componentInfo.loading = true
        const promise = componentInfo.loader()
          .then((module: any) => {
            componentInfo.component = module.default || module
            componentInfo.registered = true
            componentInfo.loading = false
          })
          .catch((error: Error) => {
            componentInfo.loading = false
            console.error(`[rari] Failed to preload component ${key}:`, error)
          })
        loadPromises.push(promise)
        break
      }
    }
  }

  if (loadPromises.length > 0)
    await Promise.all(loadPromises)
}
