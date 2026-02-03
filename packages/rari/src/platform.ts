import { existsSync } from 'node:fs'
import { dirname, join, parse } from 'node:path'
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

export function getBinaryPath(): string {
  const { packageName, binaryName } = getPlatformInfo()

  try {
    let currentDir = process.cwd()
    let workspaceRoot = null

    /* v8 ignore start - workspace search logic, tested in actual workspace */
    const rootDir = parse(currentDir).root
    while (currentDir !== rootDir && currentDir !== '') {
      const packagesDir = join(currentDir, 'packages')
      if (existsSync(packagesDir)) {
        workspaceRoot = currentDir
        break
      }
      const parentDir = dirname(currentDir)
      if (parentDir === currentDir)
        break
      currentDir = parentDir
    }

    if (workspaceRoot) {
      const packageDir = join(workspaceRoot, 'packages', packageName)
      const binaryPath = join(packageDir, 'bin', binaryName)

      if (existsSync(binaryPath))
        return binaryPath
    }
    /* v8 ignore stop */
  }
  /* v8 ignore start - error handling for workspace search */
  catch {
  }
  /* v8 ignore stop */

  /* v8 ignore start - fallback to import.meta.resolve, tested in workspace */
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
  /* v8 ignore stop */
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
