import type { ModuleAnalysis } from './directives'
import fs from 'node:fs'
import { analyzeModuleSource } from './directives'

interface CacheEntry {
  mtimeMs: number
  sourceLength: number
  analysis: ModuleAnalysis
}

export class ModuleAnalysisCache {
  private cache = new Map<string, CacheEntry>()

  get(filePath: string, source?: string): ModuleAnalysis {
    if (source !== undefined) {
      const cached = this.cache.get(filePath)
      if (cached && cached.sourceLength === source.length)
        return cached.analysis

      const analysis = analyzeModuleSource(source)
      this.cache.set(filePath, {
        mtimeMs: this.readMtimeMs(filePath),
        sourceLength: source.length,
        analysis,
      })
      return analysis
    }

    const mtimeMs = this.readMtimeMs(filePath)
    const cached = this.cache.get(filePath)
    if (cached && cached.mtimeMs === mtimeMs)
      return cached.analysis

    const code = fs.readFileSync(filePath, 'utf-8')
    const analysis = analyzeModuleSource(code)
    this.cache.set(filePath, {
      mtimeMs,
      sourceLength: code.length,
      analysis,
    })
    return analysis
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

export function filterExternalDependencies(
  importSources: readonly string[],
  nodeBuiltins: ReadonlySet<string>,
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
