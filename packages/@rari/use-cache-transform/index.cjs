// @ts-check
// CJS counterpart of index.js.

const fs = require('node:fs')
const path = require('node:path')
const process = require('node:process')

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
  const localNode = path.join(__dirname, 'use_cache_transform.node')
  if (fs.existsSync(localNode)) {
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

module.exports = nativeBinding
module.exports.default = nativeBinding
