import { execSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'

export interface PlatformInfo {
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

export function getPlatformInfo(): PlatformInfo {
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
        `Unsupported platform: ${platform}. Rari supports Linux, macOS, and Windows.`,
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
        `Unsupported architecture: ${arch}. Rari supports x64 and ARM64.`,
      )
  }

  const platformKey
    = `${normalizedPlatform}-${normalizedArch}` as keyof typeof SUPPORTED_PLATFORMS
  const packageName = SUPPORTED_PLATFORMS[platformKey]

  if (!packageName) {
    throw new Error(
      `Unsupported platform combination: ${normalizedPlatform}-${normalizedArch}. `
      + `Supported platforms: ${Object.keys(SUPPORTED_PLATFORMS).join(', ')}`,
    )
  }

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

    while (currentDir !== '/' && currentDir !== '') {
      const packagesDir = join(currentDir, 'packages')
      if (existsSync(packagesDir)) {
        workspaceRoot = currentDir
        break
      }
      currentDir = join(currentDir, '..')
    }

    if (workspaceRoot) {
      const packageDir = join(workspaceRoot, 'packages', packageName)
      const binaryPath = join(packageDir, 'bin', binaryName)

      if (existsSync(binaryPath)) {
        return binaryPath
      }
    }
  }
  catch {
  }

  try {
    const packagePath = require.resolve(`${packageName}/package.json`)
    const packageDir = packagePath.replace('/package.json', '')
    const binaryPath = join(packageDir, 'bin', binaryName)

    if (existsSync(binaryPath)) {
      return binaryPath
    }

    throw new Error(`Binary not found at ${binaryPath}`)
  }
  catch {
    throw new Error(
      `Failed to locate Rari binary for ${packageName}. `
      + `Please ensure the platform package is installed: npm install ${packageName}`,
    )
  }
}

export function validateBinary(binaryPath: string): boolean {
  try {
    const result = execSync(`"${binaryPath}" --version`, {
      encoding: 'utf8',
      timeout: 5000,
      stdio: 'pipe',
    })

    return result.trim().startsWith('rari')
  }
  catch {
    return false
  }
}

export function getInstallationInstructions(): string {
  const { packageName } = getPlatformInfo()

  return `
To install Rari for your platform, run:

  npm install ${packageName}

Or if you're using pnpm:

  pnpm add ${packageName}

Or if you're using yarn:

  yarn add ${packageName}

If you continue to have issues, you can also install from source:

  cargo install --git https://github.com/rari-build/rari
`
}

export function getSupportedPlatforms(): string[] {
  return Object.keys(SUPPORTED_PLATFORMS)
}
