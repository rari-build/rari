import fs from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const USE_CACHE_FUNCTION_REGEX = /['"]use\s+cache(?::\s*[\w-]+)?['"]/

function hasUseCacheFunction(code: string): boolean {
  return USE_CACHE_FUNCTION_REGEX.test(code)
}

let useCacheAddon: any = null

function getUseCacheTransformAddon(): any {
  if (useCacheAddon)
    return useCacheAddon

  const packageRoot = path.resolve(__dirname, '..')
  const repoRoot = path.resolve(packageRoot, '..', '..', '..')

  const possiblePaths = [
    path.join(packageRoot, 'target/release/use_cache_transform.node'),
    path.join(repoRoot, 'target/release/use_cache_transform.node'),
    path.join(packageRoot, '../../target/release/use_cache_transform.node'),
    path.join(packageRoot, 'target/debug/use_cache_transform.node'),
    path.join(repoRoot, 'target/debug/use_cache_transform.node'),
    path.join(packageRoot, '../../target/debug/use_cache_transform.node'),
    path.join(repoRoot, 'packages/rari-linux-x64/bin/use_cache_transform.node'),
    path.join(repoRoot, 'packages/rari-win32-x64/bin/use_cache_transform.node'),
  ]

  for (const addonPath of possiblePaths) {
    if (fs.existsSync(addonPath)) {
      try {
        const nodeRequire = createRequire(import.meta.url)
        useCacheAddon = nodeRequire(addonPath)

        return useCacheAddon
      }
      catch (err) {
        console.warn('[rari] Failed to load use-cache-transform addon:', err)
      }
    }
  }

  return null
}

function extractPrologueLines(code: string): string[] {
  const lines = code.split('\n')
  const prologue: string[] = []
  for (const line of lines) {
    const trimmed = line.trim()
    if (trimmed === '') {
      prologue.push(line)
      continue
    }
    if (/^['"]/.test(trimmed) && /['"];\s*$/.test(trimmed)) {
      prologue.push(line)
    }
    else {
      break
    }
  }

  return prologue
}

export function transformUseCacheModule(code: string, id: string): string | null {
  if (!hasUseCacheFunction(code)) {
    return null
  }

  const addon = getUseCacheTransformAddon()
  if (!addon) {
    return null
  }

  try {
    const result = addon.transformUseCache(code, {
      filename: id,
      hashSalt: 'rari-use-cache-v1',
      cacheKinds: ['default'],
    })

    if (result.code === code) {
      return null
    }

    const imports = []
    if (result.needsReactCache) {
      imports.push(`import { cache as $$reactCache__ } from 'react'`)
    }

    if (result.needsCacheWrapper) {
      imports.push(`import { $$cache__, encodeBoundArgs } from 'rari/runtime/cache-wrapper'`)
    }

    if (result.needsRegisterRef) {
      imports.push(`import { registerServerReference } from 'rari/runtime/react-server-dom-shim'`)
    }

    const prologueLines = extractPrologueLines(result.code)
    const importBlock = imports.length ? `${imports.join(';\n')};\n` : ''

    if (prologueLines.length) {
      const rest = result.code.split('\n').slice(prologueLines.length).join('\n').trimStart()

      return `${prologueLines.join('\n')}\n${importBlock}${rest}`
    }

    return `${importBlock}${result.code}`
  }
  catch (err) {
    throw new Error(
      `Failed to transform 'use cache' directive in ${id}: ${err instanceof Error ? err.message : String(err)}`,
    )
  }
}
