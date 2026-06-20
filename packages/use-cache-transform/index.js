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

async function loadAddon() {
  const localNode = resolve(__dirname, 'rari_use_cache_transform.node')
  if (existsSync(localNode)) {
    return require(localNode)
  }

  try {
    const platformModule = await import(platformPkg)
    return platformModule.default
  }
  catch {
    return null
  }
}

// eslint-disable-next-line antfu/no-top-level-await
const nativeBinding = await loadAddon()

export function detectUseCache(source) {
  if (!nativeBinding) {
    return false
  }

  return nativeBinding.detectUseCache(source)
}

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
