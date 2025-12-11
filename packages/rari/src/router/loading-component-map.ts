import type { LoadingEntry } from './app-types'
import path from 'node:path'

export interface LoadingComponentMapOptions {
  appDir: string
  loadingComponents: LoadingEntry[]
}

export function generateLoadingComponentMap(options: LoadingComponentMapOptions): string {
  const { loadingComponents } = options

  if (loadingComponents.length === 0) {
    return `// No loading components found
export const loadingComponentModules = {}

if (typeof globalThis !== 'undefined') {
  globalThis.__rari_loading_components = new Map()
}
`
  }

  const moduleEntries: string[] = []

  for (const loading of loadingComponents) {
    const { componentId, filePath } = loading

    const importPath = `./${path.posix.join('app', filePath.replace(/\\/g, '/'))}`

    moduleEntries.push(`  '${componentId}': () => import('${importPath}')`)
  }

  const code = `// Auto-generated loading component module map

export const loadingComponentModules = {
${moduleEntries.join(',\n')}
}

if (typeof globalThis !== 'undefined') {
  globalThis.__rari_loading_components = new Map(Object.entries(loadingComponentModules))
}
`

  return code
}

export function getLoadingComponentMapPath(outDir: string): string {
  return path.join(outDir, 'loading-component-map.js')
}
