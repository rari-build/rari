import type { ModuleAnalysis } from './directives'
import fs from 'node:fs'
import { analyzeModuleSource } from './directives'

export const NODE_BUILTIN_MODULES = new Set([
  'fs',
  'path',
  'os',
  'crypto',
  'util',
  'stream',
  'events',
  'process',
  'buffer',
  'url',
  'querystring',
  'zlib',
  'http',
  'https',
  'net',
  'tls',
  'child_process',
  'cluster',
  'worker_threads',
])

export function isNodeBuiltinModule(moduleName: string): boolean {
  return NODE_BUILTIN_MODULES.has(moduleName)
}

export function hasNodeImportsFromAnalysis(analysis: ModuleAnalysis): boolean {
  for (const importPath of analysis.importSources) {
    if (importPath.startsWith('node:') || isNodeBuiltinModule(importPath))
      return true
  }

  return false
}

export function filterExternalDependencies(
  importSources: readonly string[],
  nodeBuiltins: ReadonlySet<string> = NODE_BUILTIN_MODULES,
): string[] {
  const dependencies: string[] = []

  for (const importPath of importSources) {
    if (
      !importPath.startsWith('.')
      && !importPath.startsWith('/')
      && !importPath.startsWith('node:')
      && !nodeBuiltins.has(importPath)
    ) {
      dependencies.push(importPath)
    }
  }

  return [...new Set(dependencies)]
}

export function filterRelativeImportSources(importSources: readonly string[]): string[] {
  const imports: string[] = []

  for (const importPath of importSources) {
    if (importPath.startsWith('./') || importPath.startsWith('../') || importPath.startsWith('@/'))
      imports.push(importPath)
  }

  return imports
}

interface CacheEntry {
  mtimeMs: number
  sourceHash: number
  source: string
  analysis: ModuleAnalysis
}

function hashSource(source: string): number {
  let hash = 5381
  for (let i = 0; i < source.length; i++)
    hash = ((hash << 5) + hash) ^ source.charCodeAt(i)

  return hash >>> 0
}

export class ModuleAnalysisCache {
  private cache = new Map<string, CacheEntry>()

  get(filePath: string, source?: string): ModuleAnalysis {
    const mtimeMs = this.readMtimeMs(filePath)
    const cached = this.cache.get(filePath)

    if (source === undefined) {
      if (cached && cached.mtimeMs === mtimeMs)
        return cached.analysis

      source = fs.readFileSync(filePath, 'utf-8')
    }
    else {
      const sourceHash = hashSource(source)
      if (cached && cached.sourceHash === sourceHash)
        return cached.analysis
    }

    const sourceHash = hashSource(source)
    const analysis = analyzeModuleSource(source)
    this.cache.set(filePath, {
      mtimeMs,
      sourceHash,
      source,
      analysis,
    })
    return analysis
  }

  getSource(filePath: string): string {
    this.get(filePath)
    return this.cache.get(filePath)!.source
  }

  invalidate(filePath: string): void {
    this.cache.delete(filePath)
  }

  clear(): void {
    this.cache.clear()
  }

  private readMtimeMs(filePath: string): number {
    try {
      return fs.statSync(filePath).mtimeMs
    }
    catch {
      return -1
    }
  }
}
