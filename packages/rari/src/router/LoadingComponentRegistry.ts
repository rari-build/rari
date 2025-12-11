import type React from 'react'

export class LoadingComponentRegistry {
  private components: Map<string, React.ComponentType> = new Map()
  private loadingPromises: Map<string, Promise<React.ComponentType | null>> = new Map()
  private loadingModules: Map<string, () => Promise<{ default: React.ComponentType }>> | null = null
  private loadingModulesInitialized = false

  register(routePath: string, component: React.ComponentType): void {
    this.components.set(routePath, component)
  }

  getLoadingComponent(routePath: string): React.ComponentType | null {
    if (this.components.has(routePath)) {
      return this.components.get(routePath)!
    }

    const segments = routePath.split('/').filter(Boolean)
    for (let i = segments.length - 1; i >= 0; i--) {
      const parentPath = `/${segments.slice(0, i).join('/')}`
      if (this.components.has(parentPath)) {
        return this.components.get(parentPath)!
      }
    }

    if (this.components.has('/')) {
      return this.components.get('/')!
    }

    return null
  }

  async loadComponent(routePath: string): Promise<React.ComponentType | null> {
    const existing = this.components.get(routePath)
    if (existing) {
      return existing
    }

    const existingPromise = this.loadingPromises.get(routePath)
    if (existingPromise) {
      return existingPromise
    }

    const loadPromise = this.loadComponentFromManifest(routePath)
    this.loadingPromises.set(routePath, loadPromise)

    try {
      const component = await loadPromise
      if (component) {
        this.register(routePath, component)
      }
      return component
    }
    catch (error) {
      console.warn(`[LoadingRegistry] Failed to load component for ${routePath}:`, error)
      return null
    }
    finally {
      this.loadingPromises.delete(routePath)
    }
  }

  private async loadComponentFromManifest(routePath: string): Promise<React.ComponentType | null> {
    if (!this.loadingModulesInitialized) {
      this.loadingModules = (globalThis as any).__rari_loading_components
      this.loadingModulesInitialized = true

      if (!this.loadingModules) {
        console.warn('[LoadingRegistry] No loading component modules available')
      }
    }

    if (!this.loadingModules) {
      return null
    }

    const componentId = `loading:${routePath}`
    const exactLoader = this.loadingModules.get(componentId)
    if (exactLoader) {
      try {
        const module = await exactLoader()
        if (module && module.default && typeof module.default === 'function') {
          return module.default
        }
        console.warn(`[LoadingRegistry] Invalid component for ${routePath}: module.default is not a function`)
      }
      catch (error) {
        console.warn(`[LoadingRegistry] Failed to load exact match for ${routePath}:`, error)
      }
    }

    const segments = routePath.split('/').filter(Boolean)
    for (let i = segments.length - 1; i >= 0; i--) {
      const parentPath = `/${segments.slice(0, i).join('/')}`
      const parentComponentId = `loading:${parentPath}`
      const parentLoader = this.loadingModules.get(parentComponentId)
      if (parentLoader) {
        try {
          const module = await parentLoader()
          return module.default
        }
        catch (error) {
          console.warn(`[LoadingRegistry] Failed to load parent match for ${parentPath}:`, error)
        }
      }
    }

    const rootComponentId = 'loading:/'
    const rootLoader = this.loadingModules.get(rootComponentId)
    if (rootLoader) {
      try {
        const module = await rootLoader()
        return module.default
      }
      catch (error) {
        console.warn('[LoadingRegistry] Failed to load root loading component:', error)
      }
    }

    return null
  }

  hasLoadingComponent(routePath: string): boolean {
    return this.getLoadingComponent(routePath) !== null
  }

  clear(): void {
    this.components.clear()
    this.loadingPromises.clear()
  }

  size(): number {
    return this.components.size
  }
}
