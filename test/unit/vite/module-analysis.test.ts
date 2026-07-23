import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import {
  analyzeModuleSource,
  getDirectives,
  hasDefaultExport,
  hasTopLevelUseClientDirective,
  hasTopLevelUseServerDirective,
} from '@rari/vite/analysis/directives'
import {
  collectClientComponentPaths,
  filterExternalDependencies,
  hasNodeImportsFromAnalysis,
  invalidateModuleCachePath,
  isNodeBuiltinModule,
  ModuleAnalysisCache,
  resolveModuleCachePath,
} from '@rari/vite/analysis/module-cache'
import { describe, expect, it, vi } from 'vite-plus/test'

describe('analyzeModuleSource', () => {
  it('detects directives, exports, and imports in one pass', () => {
    const source = `"use client"

import React from 'react'
import { useState } from 'react'

export default function App() {
  return <div>{useState(0)}</div>
}
`

    const analysis = analyzeModuleSource(source)

    expect(analysis.directives.hasUseClient).toBe(true)
    expect(analysis.topLevelUseClient).toBe(true)
    expect(analysis.hasDefaultExport).toBe(true)
    expect(analysis.hasComponentExport).toBe(true)
    expect(analysis.importSources).toEqual(['react'])
  })

  it('ignores import.meta and still collects real imports', () => {
    const source = `import React from 'react'

if (import.meta.hot) {
  import.meta.hot.accept()
}

export default function App() {
  return import.meta.env.DEV ? <div /> : null
}
`

    const analysis = analyzeModuleSource(source)

    expect(analysis.importSources).toEqual(['react'])
    expect(analysis.hasDefaultExport).toBe(true)
  })

  it('detects dynamic imports', () => {
    const source = `
export async function load() {
  const mod = await import('@acme/pkg')
  return mod.default
}
`

    const analysis = analyzeModuleSource(source)
    expect(analysis.importSources).toContain('@acme/pkg')
  })

  it('matches legacy directive helpers', () => {
    const source = `"use server"

export async function action() {}
`

    const analysis = analyzeModuleSource(source)
    expect(getDirectives(source)).toEqual(analysis.directives)
    expect(hasTopLevelUseServerDirective(source)).toBe(analysis.topLevelUseServer)
    expect(hasTopLevelUseClientDirective(source)).toBe(analysis.topLevelUseClient)
    expect(hasDefaultExport(source)).toBe(analysis.hasDefaultExport)
  })

  it('filters external dependencies including dynamic imports', () => {
    const source = `
import fs from 'node:fs'
import react from 'react'
const mod = await import('@acme/dynamic')
`
    const analysis = analyzeModuleSource(source)
    const external = filterExternalDependencies(analysis.importSources)

    expect(external).toEqual(['react', '@acme/dynamic'])
  })

  it('excludes @/ aliased imports from external dependencies', () => {
    const source = `
import react from 'react'
import { Button } from '@/components/Button'
`
    const analysis = analyzeModuleSource(source)
    const external = filterExternalDependencies(analysis.importSources)

    expect(external).toEqual(['react'])
  })
})

describe('moduleAnalysisCache', () => {
  it('reuses analysis until mtime changes', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-analysis-'))
    const filePath = path.join(dir, 'Component.tsx')
    fs.writeFileSync(filePath, `"use client"\nexport default function C() {}\n`)

    const cache = new ModuleAnalysisCache()
    const first = cache.get(filePath)
    const second = cache.get(filePath)

    expect(first).toBe(second)

    fs.writeFileSync(filePath, `"use server"\nexport async function action() {}\n`)
    const third = cache.get(filePath)

    expect(third.directives.hasUseServer).toBe(true)
    expect(third.topLevelUseServer).toBe(true)

    fs.rmSync(dir, { recursive: true, force: true })
  })

  it('invalidates cached entries', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-analysis-'))
    const filePath = path.join(dir, 'Component.tsx')
    fs.writeFileSync(filePath, `export default function C() {}\n`)

    const cache = new ModuleAnalysisCache()
    const first = cache.get(filePath)
    cache.invalidate(filePath)
    fs.writeFileSync(filePath, `"use client"\nexport default function C() {}\n`)
    const second = cache.get(filePath)

    expect(first.hasDefaultExport).toBe(true)
    expect(second.topLevelUseClient).toBe(true)

    fs.rmSync(dir, { recursive: true, force: true })
  })

  it('re-analyzes same-length inline source edits', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-analysis-'))
    const filePath = path.join(dir, 'Component.tsx')
    fs.writeFileSync(filePath, `export default function C() {}\n`)

    const targetLength = 80
    const clientSource =
      `"use client";\nexport default function ComponentA() { return 1; }\n`.padEnd(
        targetLength,
        ' ',
      )
    const serverSource =
      `"use server";\nexport async function actionHandler() { return 1; }\n`.padEnd(
        targetLength,
        ' ',
      )
    expect(clientSource.length).toBe(serverSource.length)

    const cache = new ModuleAnalysisCache()
    const clientAnalysis = cache.get(filePath, clientSource)
    const serverAnalysis = cache.get(filePath, serverSource)

    expect(clientAnalysis.topLevelUseClient).toBe(true)
    expect(serverAnalysis.topLevelUseServer).toBe(true)
    expect(clientAnalysis).not.toBe(serverAnalysis)

    fs.rmSync(dir, { recursive: true, force: true })
  })

  it('detects node imports from extracted import sources', () => {
    const source = `import fs from 'node:fs'\nimport path from 'path'\nexport default function C() {}\n`
    const analysis = analyzeModuleSource(source)

    expect(analysis.importSources).toEqual(['node:fs', 'path'])
    expect(hasNodeImportsFromAnalysis(analysis)).toBe(true)
    expect(isNodeBuiltinModule('fs')).toBe(true)
    expect(isNodeBuiltinModule('react')).toBe(false)
  })

  it('does not treat missing files as mtime cache hits', () => {
    const cache = new ModuleAnalysisCache()
    const missingPath = path.join(os.tmpdir(), `missing-${Date.now()}.tsx`)

    cache.get(missingPath, `"use client"\nexport default function C() {}\n`)

    expect(() => cache.get(missingPath)).toThrow()
  })

  it('invalidates symlink and real paths', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-analysis-'))
    const targetPath = path.join(dir, 'Component.tsx')
    const linkPath = path.join(dir, 'Component.link.tsx')

    fs.writeFileSync(targetPath, `export default function C() {}\n`)
    fs.symlinkSync(targetPath, linkPath)

    const cache = new ModuleAnalysisCache()
    const first = cache.get(linkPath)

    cache.invalidate(targetPath)

    fs.writeFileSync(targetPath, `"use client"\nexport default function C() {}\n`)
    const second = cache.get(linkPath)

    expect(first.hasDefaultExport).toBe(true)
    expect(second.topLevelUseClient).toBe(true)

    fs.rmSync(dir, { recursive: true, force: true })
  })

  it('invalidateModuleCachePath clears symlink and real keys', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-analysis-'))
    const targetPath = path.join(dir, 'Component.tsx')
    const linkPath = path.join(dir, 'Component.link.tsx')

    fs.writeFileSync(targetPath, `export default function C() {}\n`)
    fs.symlinkSync(targetPath, linkPath)

    const cache = new Map<string, string>()
    cache.set(resolveModuleCachePath(linkPath), 'client')

    invalidateModuleCachePath(cache, targetPath)

    expect(cache.has(resolveModuleCachePath(linkPath))).toBe(false)

    fs.rmSync(dir, { recursive: true, force: true })
  })

  it('skips mtime reads on inline source cache hits', () => {
    const statSpy = vi.spyOn(fs, 'statSync')
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-analysis-'))
    const filePath = path.join(dir, 'Component.tsx')
    fs.writeFileSync(filePath, `export default function C() {}\n`)

    const source = `"use client"\nexport default function C() {}\n`
    const cache = new ModuleAnalysisCache()
    cache.get(filePath, source)
    const statCallsAfterFirst = statSpy.mock.calls.length

    cache.get(filePath, source)

    expect(statSpy.mock.calls.length).toBe(statCallsAfterFirst)

    statSpy.mockRestore()
    fs.rmSync(dir, { recursive: true, force: true })
  })

  it('returns cached source after analysis', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-analysis-'))
    const filePath = path.join(dir, 'Component.tsx')
    fs.writeFileSync(filePath, `export default function C() {}\n`)

    const cache = new ModuleAnalysisCache()
    cache.get(filePath)

    expect(cache.getSource(filePath)).toBe(`export default function C() {}\n`)

    fs.rmSync(dir, { recursive: true, force: true })
  })

  it('collectClientComponentPaths finds use client files in a directory tree', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-analysis-'))
    const srcDir = path.join(dir, 'src')
    fs.mkdirSync(path.join(srcDir, 'app'), { recursive: true })
    fs.writeFileSync(
      path.join(srcDir, 'app', 'template.tsx'),
      `"use client"\nexport default function T() {}\n`,
    )
    fs.writeFileSync(path.join(srcDir, 'app', 'page.tsx'), `export default function P() {}\n`)

    const cache = new ModuleAnalysisCache()
    const clientPaths = collectClientComponentPaths([srcDir], cache)

    expect(clientPaths).toEqual([path.join(srcDir, 'app', 'template.tsx')])

    fs.rmSync(dir, { recursive: true, force: true })
  })
})
