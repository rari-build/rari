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
    case 'aix':
    case 'android':
    case 'cygwin':
    case 'freebsd':
    case 'haiku':
    case 'openbsd':
    case 'sunos':
    case 'netbsd':
      throw new Error(`Unsupported platform: ${platform}. rari supports Linux, macOS, and Windows.`)
    default: {
      const _exhaustive: never = platform
      throw new Error(
        `Unsupported platform: ${String(_exhaustive)}. rari supports Linux, macOS, and Windows.`,
      )
    }
  }

  let normalizedArch: string
  switch (arch) {
    case 'x64':
      normalizedArch = 'x64'
      break
    case 'arm64':
      normalizedArch = 'arm64'
      break
    case 'arm':
    case 'ia32':
    case 'loong64':
    case 'mips':
    case 'mipsel':
    case 'ppc64':
    case 'riscv64':
    case 's390x':
      throw new Error(`Unsupported architecture: ${arch}. rari supports x64 and ARM64.`)
    default: {
      const _exhaustive: never = arch
      throw new Error(
        `Unsupported architecture: ${String(_exhaustive)}. rari supports x64 and ARM64.`,
      )
    }
  }

  const platformKey = `${normalizedPlatform}-${normalizedArch}`
  if (!isSupportedPlatformKey(platformKey)) {
    throw new Error(
      `Unsupported platform combination: ${normalizedPlatform}-${normalizedArch}. ` +
        `Supported platforms: ${Object.keys(SUPPORTED_PLATFORMS).join(', ')}`,
    )
  }

  const packageName = SUPPORTED_PLATFORMS[platformKey]

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
