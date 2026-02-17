import { BACKSLASH_REGEX, SRC_PREFIX_REGEX } from '../../shared/regex-constants'
import { GlobalWithRari, ModuleData } from './types'

export async function preloadComponentsFromModules(modules: Map<string, ModuleData>): Promise<void> {
  const clientComponents = (globalThis as unknown as GlobalWithRari)['~clientComponents'] || {}
  const loadPromises: Promise<any>[] = []

  for (const [, moduleData] of modules) {
    const lookupKeys = [
      moduleData.id,
      `${moduleData.id}#${moduleData.name || 'default'}`,
      moduleData.id.replace(SRC_PREFIX_REGEX, ''),
      moduleData.id.replace(BACKSLASH_REGEX, '/'),
    ]

    for (const key of lookupKeys) {
      const componentInfo = clientComponents[key]
      if (componentInfo?.loader && !componentInfo.component && !componentInfo.loading) {
        componentInfo.loading = true
        componentInfo.loadPromise = componentInfo.loader()
          .then((module: any) => {
            componentInfo.component = module.default || module
            componentInfo.registered = true
            componentInfo.loading = false
          })
          .catch((error: Error) => {
            componentInfo.loading = false
            componentInfo.loadPromise = undefined
            console.error(`[rari] Failed to preload component ${key}:`, error)
          })
        loadPromises.push(componentInfo.loadPromise)
        break
      }
    }
  }

  if (loadPromises.length > 0)
    await Promise.all(loadPromises)
}
