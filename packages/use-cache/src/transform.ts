import type { NativeAddon } from './native'
import { requireNativeAddon } from './native'

const USE_CACHE_FUNCTION_REGEX = /['"]use\s+cache(?::\s*[\w-]+)?['"]/
const DIRECTIVE_PROLOGUE_REGEX = /^['"][^'"]+['"];?\s*$/

function extractPrologueLines(code: string) {
  const lines = code.split('\n')
  const prologue: string[] = []
  for (const line of lines) {
    const trimmed = line.trim()
    if (trimmed === '') {
      prologue.push(line)
      continue
    }
    if (DIRECTIVE_PROLOGUE_REGEX.test(trimmed)) prologue.push(line)
    else break
  }

  return prologue
}

export interface UseCacheTransformOptions {
  readonly hashSalt?: string
  readonly cacheKinds?: readonly string[]
}

export function transformUseCacheModule(
  code: string,
  id: string,
  options: UseCacheTransformOptions = {},
): string | null {
  if (!USE_CACHE_FUNCTION_REGEX.test(code)) return null

  const native: NativeAddon = requireNativeAddon()

  try {
    const result = native.transformUseCache(code, {
      filename: id,
      hashSalt: options.hashSalt ?? 'rari-use-cache-v1',
      cacheKinds: options.cacheKinds ?? ['default'],
    })

    if (result.code === code) return null

    const imports = []
    if (result.needsReactCache) imports.push(`import { cache as $$reactCache__ } from 'react'`)

    if (result.needsCacheWrapper)
      imports.push(`import { $$cache__ } from '@rari/use-cache/runtime/cache-wrapper'`)

    if (result.needsRegisterRef)
      imports.push(`import { registerServerReference } from 'react-server-dom-rari/server'`)

    const prologueLines = extractPrologueLines(result.code)
    const importBlock = imports.length ? `${imports.join(';\n')};\n` : ''

    if (prologueLines.length) {
      const rest = result.code.split('\n').slice(prologueLines.length).join('\n').trimStart()

      return `${prologueLines.join('\n')}\n${importBlock}${rest}`
    }

    return `${importBlock}${result.code}`
  } catch (err) {
    throw new Error(
      `Failed to transform 'use cache' directive in ${id}: ${err instanceof Error ? err.message : String(err)}`,
      { cause: err },
    )
  }
}
