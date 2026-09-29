import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

interface PlatformInfo {
  platform: string
  arch: string
  packageName: string
  binaryName: string
}

const SUPPORTED_PLATFORMS = {
  'linux-x64': 'rari-linux-x64',
  'linux-arm64': 'rari-linux-arm64',
  'darwin-x64': 'rari-darwin-x64',
  'darwin-arm64': 'rari-darwin-arm64',
  'win32-arm64': 'rari-win32-arm64',
  'win32-x64': 'rari-win32-x64',
} as const

function isSupportedPlatformKey(key: string): key is keyof typeof SUPPORTED_PLATFORMS {
  return Object.hasOwn(SUPPORTED_PLATFORMS, key)
}

function getPlatformInfo(): PlatformInfo {
  const platform = process.platform
  const arch = process.arch

  if (platform !== 'darwin' && platform !== 'linux' && platform !== 'win32') {
    throw new Error(`Unsupported platform: ${platform}. rari supports Linux, macOS, and Windows.`)
  }

  if (arch !== 'x64' && arch !== 'arm64') {
    throw new Error(`Unsupported architecture: ${arch}. rari supports x64 and ARM64.`)
  }

  const platformKey = `${platform}-${arch}`
  if (!isSupportedPlatformKey(platformKey)) {
    throw new Error(
      `Unsupported platform combination: ${platform}-${arch}. ` +
        `Supported platforms: ${Object.keys(SUPPORTED_PLATFORMS).join(', ')}`,
    )
  }

  const packageName = SUPPORTED_PLATFORMS[platformKey]
  const binaryName = platform === 'win32' ? 'rari.exe' : 'rari'

  return {
    platform,
    arch,
    packageName,
    binaryName,
  }
}

let cachedBinaryPath: string | null = null

function resolveBinaryPath(): string {
  const { packageName, binaryName } = getPlatformInfo()

  const selfDir = dirname(fileURLToPath(import.meta.url))
  let searchDir = selfDir
  while (searchDir !== dirname(searchDir)) {
    if (existsSync(join(searchDir, 'pnpm-workspace.yaml'))) {
      const localBinary = join(searchDir, 'packages', packageName, 'bin', binaryName)
      if (existsSync(localBinary)) return localBinary
      break
    }
    searchDir = dirname(searchDir)
  }

  try {
    const packagePath = import.meta.resolve(`${packageName}/package.json`)
    const packageDir = fileURLToPath(new URL('.', packagePath))
    const binaryPath = join(packageDir, 'bin', binaryName)

    if (existsSync(binaryPath)) return binaryPath

    throw new Error(`Binary not found at ${binaryPath}`)
  } catch {
    throw new Error(
      `Failed to locate rari binary for ${packageName}. ` +
        `Please ensure the platform package is installed: npm install ${packageName}`,
    )
  }
}

export function getBinaryPath(): string {
  if (cachedBinaryPath != null && cachedBinaryPath !== '') return cachedBinaryPath

  cachedBinaryPath = resolveBinaryPath()
  return cachedBinaryPath
}

export function getInstallationInstructions(): string {
  const { packageName } = getPlatformInfo()

  return `
To install rari for your platform, run:

  npm install ${packageName}

Or if you're using pnpm:

  pnpm add ${packageName}

Or if you're using yarn:

  yarn add ${packageName}

If you continue to have issues, you can also install from source:

  cargo install --git https://github.com/rari-build/rari
`
}
