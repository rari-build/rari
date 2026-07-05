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

function getPlatformInfo(): PlatformInfo {
  const platform = process.platform
  const arch = process.arch

  let normalizedPlatform: string
  switch (platform) {
    case 'darwin':
      normalizedPlatform = 'darwin'
      break
    case 'linux':
      normalizedPlatform = 'linux'
      break
    case 'win32':
      normalizedPlatform = 'win32'
      break
    default:
      throw new Error(
        `Unsupported platform: ${platform}. rari supports Linux, macOS, and Windows.`,
      )
  }

  let normalizedArch: string
  switch (arch) {
    case 'x64':
      normalizedArch = 'x64'
      break
    case 'arm64':
      normalizedArch = 'arm64'
      break
    default:
      throw new Error(
        `Unsupported architecture: ${arch}. rari supports x64 and ARM64.`,
      )
  }

  const platformKey
    = `${normalizedPlatform}-${normalizedArch}` as keyof typeof SUPPORTED_PLATFORMS
  const packageName = SUPPORTED_PLATFORMS[platformKey]

  /* v8 ignore start - defensive check, all valid combinations are in SUPPORTED_PLATFORMS */
  if (!packageName) {
    throw new Error(
      `Unsupported platform combination: ${normalizedPlatform}-${normalizedArch}. `
      + `Supported platforms: ${Object.keys(SUPPORTED_PLATFORMS).join(', ')}`,
    )
  }
  /* v8 ignore stop */

  const binaryName = normalizedPlatform === 'win32' ? 'rari.exe' : 'rari'

  return {
    platform: normalizedPlatform,
    arch: normalizedArch,
    packageName,
    binaryName,
  }
}

let cachedBinaryPath: string | null = null

function resolveBinaryPath(): string {
  const { packageName, binaryName } = getPlatformInfo()

  const selfDir = dirname(fileURLToPath(import.meta.url))
  let searchDir = selfDir
  while (true) {
    if (existsSync(join(searchDir, 'pnpm-workspace.yaml'))) {
      const localBinary = join(searchDir, 'packages', packageName, 'bin', binaryName)
      if (existsSync(localBinary))
        return localBinary
      break
    }
    const parent = dirname(searchDir)
    if (parent === searchDir)
      break
    searchDir = parent
  }

  try {
    const packagePath = import.meta.resolve(`${packageName}/package.json`)
    const packageDir = fileURLToPath(new URL('.', packagePath))
    const binaryPath = join(packageDir, 'bin', binaryName)

    if (existsSync(binaryPath))
      return binaryPath

    throw new Error(`Binary not found at ${binaryPath}`)
  }
  catch {
    throw new Error(
      `Failed to locate rari binary for ${packageName}. `
      + `Please ensure the platform package is installed: npm install ${packageName}`,
    )
  }
}

export function getBinaryPath(): string {
  if (cachedBinaryPath)
    return cachedBinaryPath

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
