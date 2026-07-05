import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { analyzeModuleSource, getDirectives, hasDefaultExport, hasTopLevelUseClientDirective, hasTopLevelUseServerDirective } from '@rari/vite/directives'
import { filterExternalDependencies, ModuleAnalysisCache } from '@rari/vite/module-analysis-cache'
import { describe, expect, it } from 'vite-plus/test'

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
    const external = filterExternalDependencies(analysis.importSources, new Set(['fs']))

    expect(external).toEqual(['react', '@acme/dynamic'])
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
})
