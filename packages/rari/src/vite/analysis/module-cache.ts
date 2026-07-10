import type { ModuleAnalysis } from './directives'
import fs from 'node:fs'
import { builtinModules } from 'node:module'
import path from 'node:path'
import { analyzeModuleSource } from './directives'
import { collectSourceFilePaths } from './source-walker'

export const NODE_BUILTIN_MODULES: ReadonlySet<string> = new Set(builtinModules)

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
      && !importPath.startsWith('@/')
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
  source: string
  analysis: ModuleAnalysis
}

export function resolveModuleCachePath(filePath: string): string {
  try {
    return fs.realpathSync(filePath)
  }
  catch {
    return path.resolve(filePath)
  }
}

export function invalidateModuleCachePath(
  cache: Map<string, unknown>,
  filePath: string,
): void {
  cache.delete(filePath)
  try {
    cache.delete(fs.realpathSync(filePath))
  }
  catch {
    cache.delete(path.resolve(filePath))
  }
}

export function collectClientComponentPaths(
  dirs: readonly string[],
  cache: ModuleAnalysisCache,
): string[] {
  const paths: string[] = []

  for (const filePath of collectSourceFilePaths(dirs)) {
    try {
      if (cache.get(filePath).directives.hasUseClient)
        paths.push(filePath)
    }
    catch {}
  }

  return paths
}

export class ModuleAnalysisCache {
  private cache = new Map<string, CacheEntry>()

  get(filePath: string, source?: string): ModuleAnalysis {
    const cacheKey = resolveModuleCachePath(filePath)
    const cached = this.cache.get(cacheKey)

    if (source !== undefined) {
      if (cached && cached.source === source)
        return cached.analysis
    }
    else if (cached) {
      const mtimeMs = this.readMtimeMs(cacheKey)
      if (mtimeMs >= 0 && cached.mtimeMs === mtimeMs)
        return cached.analysis
    }

    if (source === undefined)
      source = fs.readFileSync(cacheKey, 'utf-8')

    const mtimeMs = this.readMtimeMs(cacheKey)
    const analysis = analyzeModuleSource(source)
    this.cache.set(cacheKey, {
      mtimeMs,
      source,
      analysis,
    })
    return analysis
  }

  getSource(filePath: string): string {
    this.get(filePath)
    return this.cache.get(resolveModuleCachePath(filePath))!.source
  }

  invalidate(filePath: string): void {
    invalidateModuleCachePath(this.cache, filePath)
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
