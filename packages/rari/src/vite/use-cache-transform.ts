import type { NativeAddon } from '@rari/use-cache-transform'
import nativeAddon, { transformUseCache } from '@rari/use-cache-transform'

const USE_CACHE_FUNCTION_REGEX = /['"]use\s+cache(?::\s*[\w-]+)?['"]/

function hasUseCacheFunction(code: string): boolean {
  return USE_CACHE_FUNCTION_REGEX.test(code)
}

let addon: NativeAddon | null = null
let addonLoadAttempted = false

function getAddon(): NativeAddon | null {
  if (addonLoadAttempted) {
    return addon
  }
  addonLoadAttempted = true
  addon = nativeAddon
  return addon
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

  console.error(`[use-cache-transform] found 'use cache' directive in ${id}`)

  const native = getAddon()
  if (!native) {
    console.error(`[use-cache-transform] NO ADDON — skipping transform for ${id}`)
    return null
  }

  try {
    const result = transformUseCache(code, {
      filename: id,
      hashSalt: 'rari-use-cache-v1',
      cacheKinds: ['default'],
    })

    if (result.code === code) {
      console.error(`[use-cache-transform] addon returned unchanged code for ${id}`)
      return null
    }

    console.error(`[use-cache-transform] transformed ${id} (needsCacheWrapper=${result.needsCacheWrapper}, needsRegisterRef=${result.needsRegisterRef})`)

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
    console.error(`[use-cache-transform] addon threw for ${id}:`, err)
    throw new Error(
      `Failed to transform 'use cache' directive in ${id}: ${err instanceof Error ? err.message : String(err)}`,
    )
  }
}
