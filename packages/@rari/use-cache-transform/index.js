// @ts-check
// ESM loader for @rari/use-cache-transform.
//
// Resolution order:
//   1. require('./use_cache_transform.node') — local dev build, produced by
//      `just build-addon-dev`. Used when the user has built the addon for
//      their host platform and dropped the artifact next to this file.
//   2. require('@rari/use-cache-transform-{platform}-{arch}') — published
//      npm package, installed via `optionalDependencies`. Used in
//      production installs of `rari` from the npm registry.

import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, resolve } from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

const require = createRequire(import.meta.url)
const __dirname = dirname(fileURLToPath(import.meta.url))

/** @type {Record<string, string>} */
const PLATFORM_PACKAGES = {
  'darwin-arm64': '@rari/use-cache-transform-darwin-arm64',
  'darwin-x64': '@rari/use-cache-transform-darwin-x64',
  'linux-arm64': '@rari/use-cache-transform-linux-arm64',
  'linux-x64': '@rari/use-cache-transform-linux-x64',
  'win32-arm64': '@rari/use-cache-transform-win32-arm64',
  'win32-x64': '@rari/use-cache-transform-win32-x64',
}

const key = `${process.platform}-${process.arch}`
const platformPkg = PLATFORM_PACKAGES[key]
if (!platformPkg) {
  throw new Error(
    `@rari/use-cache-transform: unsupported platform ${key}. `
    + `Supported: ${Object.keys(PLATFORM_PACKAGES).join(', ')}.`,
  )
}

function loadAddon() {
  const localNode = resolve(__dirname, 'use_cache_transform.node')
  if (existsSync(localNode)) {
    return require(localNode)
  }

  try {
    return require(platformPkg)
  }
  catch {
    return null
  }
}

/** @type {import('./index.d.ts').NativeAddon | null} */
const nativeBinding = loadAddon()

/**
 * Detects if the source code contains useCache calls.
 * @param {string} source
 * @returns boolean
 */
export function detectUseCache(source) {
  if (!nativeBinding) {
    return false
  }

  return nativeBinding.detectUseCache(source)
}

/**
 * Transforms the source code to use the cache.
 * @param {string} source
 * @param {import('./index.d.ts').TransformOptions} options
 * @returns TransformResult
 */
export function transformUseCache(source, options) {
  if (!nativeBinding) {
    return {
      code: source,
      needsReactCache: false,
      needsCacheWrapper: false,
      needsRegisterRef: false,
    }
  }

  return nativeBinding.transformUseCache(source, options)
}

export default nativeBinding
