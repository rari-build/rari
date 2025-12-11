import type { Plugin } from 'rolldown-vite'
import fs from 'node:fs'
import path from 'node:path'

interface LoadingComponentEntry {
  path: string
  filePath: string
  componentId: string
}

export function scanForLoadingComponents(appDir: string): LoadingComponentEntry[] {
  const loadingComponents: LoadingComponentEntry[] = []

  function scanDirectory(dir: string, routePath: string) {
    if (!fs.existsSync(dir)) {
      return
    }

    const entries = fs.readdirSync(dir, { withFileTypes: true })

    const loadingFile = entries.find(
      entry => entry.isFile() && entry.name === 'loading.tsx',
    )

    if (loadingFile) {
      const fullPath = path.join(dir, loadingFile.name)
      const relativePath = path.relative(appDir, fullPath)
      const componentId = `loading:${routePath}`

      loadingComponents.push({
        path: routePath,
        filePath: relativePath,
        componentId,
      })
    }

    for (const entry of entries) {
      if (entry.isDirectory() && !entry.name.startsWith('.') && !entry.name.startsWith('_')) {
        const subRoutePath = routePath === '/' ? `/${entry.name}` : `${routePath}/${entry.name}`
        scanDirectory(path.join(dir, entry.name), subRoutePath)
      }
    }
  }

  scanDirectory(appDir, '/')
  return loadingComponents
}

export function generateLoadingComponentMapContent(
  loadingComponents: LoadingComponentEntry[],
  projectRoot: string,
): string {
  const imports = loadingComponents
    .map((entry) => {
      const absolutePath = path.join(projectRoot, 'src', 'app', entry.filePath)
      const relativePath = path.relative(projectRoot, absolutePath)
      return `  '${entry.componentId}': () => import('/${relativePath}?loading-component')`
    })
    .join(',\n')

  const content = `// Auto-generated loading component module map

export const loadingComponentModules = {
${imports}
}

if (typeof globalThis !== 'undefined') {
  globalThis.__rari_loading_components = new Map(Object.entries(loadingComponentModules))
}
`

  return content
}

export function createLoadingComponentPlugin(): Plugin {
  let projectRoot: string
  let loadingComponents: LoadingComponentEntry[] = []

  return {
    name: 'rari-loading-components',

    configResolved(config) {
      projectRoot = config.root
    },

    buildStart() {
      const appDir = path.join(projectRoot, 'src', 'app')
      if (!fs.existsSync(appDir)) {
        return
      }

      loadingComponents = scanForLoadingComponents(appDir)
    },

    resolveId(id) {
      if (id === 'virtual:loading-component-map') {
        return id
      }
      return null
    },

    load(id) {
      if (id === 'virtual:loading-component-map') {
        return generateLoadingComponentMapContent(loadingComponents, projectRoot)
      }
      return null
    },

    transform(code, id) {
      if (id.includes('loading.tsx') && id.includes('?loading-component')) {
        if (!code.trim().startsWith('\'use client\'') && !code.trim().startsWith('"use client"')) {
          return `'use client';\n\n${code}`
        }
      }
      return null
    },
  }
}
