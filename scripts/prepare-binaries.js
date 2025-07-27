#!/usr/bin/env node

import { execSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import colors from 'picocolors'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const projectRoot = path.resolve(__dirname, '..')

const args = process.argv.slice(2)
const buildAll = args.includes('--all')
const currentPlatformOnly = !buildAll

const TARGETS = [
  {
    target: 'x86_64-unknown-linux-gnu',
    platform: 'linux-x64',
    binaryName: 'rari',
    packageDir: 'packages/rari-linux-x64',
  },
  {
    target: 'aarch64-unknown-linux-gnu',
    platform: 'linux-arm64',
    binaryName: 'rari',
    packageDir: 'packages/rari-linux-arm64',
  },
  {
    target: 'x86_64-apple-darwin',
    platform: 'darwin-x64',
    binaryName: 'rari',
    packageDir: 'packages/rari-darwin-x64',
  },
  {
    target: 'aarch64-apple-darwin',
    platform: 'darwin-arm64',
    binaryName: 'rari',
    packageDir: 'packages/rari-darwin-arm64',
  },
  {
    target: 'x86_64-pc-windows-msvc',
    platform: 'win32-x64',
    binaryName: 'rari.exe',
    packageDir: 'packages/rari-win32-x64',
  },
]

function getCurrentPlatformTarget() {
  const platform = process.platform
  const arch = process.arch

  for (const target of TARGETS) {
    const [targetPlatform, targetArch] = target.platform.split('-')

    if (
      (platform === 'darwin' && targetPlatform === 'darwin')
      || (platform === 'linux' && targetPlatform === 'linux')
      || (platform === 'win32' && targetPlatform === 'win32')
    ) {
      if (
        (arch === 'x64' && targetArch === 'x64')
        || (arch === 'arm64' && targetArch === 'arm64')
      ) {
        return target
      }
    }
  }

  return null
}

function log(message) {
  console.warn(`${colors.cyan('➜')} ${message}`)
}

function logSuccess(message) {
  console.warn(`${colors.green('✓')} ${message}`)
}

function logError(message) {
  console.error(`${colors.red('✗')} ${message}`)
}

function logWarning(message) {
  console.warn(`${colors.yellow('⚠')} ${message}`)
}

function checkRustInstalled() {
  try {
    execSync('cargo --version', { stdio: 'pipe' })
    logSuccess('Rust/Cargo is installed')
  }
  catch {
    logError('Rust/Cargo is not installed')
    logError('Please install Rust: https://rustup.rs/')
    process.exit(1)
  }
}

function installTarget(target) {
  try {
    log(`Installing Rust target: ${target}`)
    execSync(`rustup target add ${target}`, { stdio: 'pipe' })
    logSuccess(`Installed target: ${target}`)
  }
  catch (error) {
    logWarning(`Failed to install target ${target}: ${error.message}`)
    logWarning('You may need to install additional system dependencies')
  }
}

function buildBinary(target) {
  try {
    log(`Building binary for ${target}`)

    const buildCommand = `cargo build --release --target ${target} --bin rari`
    const buildOptions = {
      cwd: projectRoot,
      stdio: 'pipe',
      env: {
        ...process.env,
        ...(target === 'aarch64-unknown-linux-gnu' && {
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER:
            'aarch64-linux-gnu-gcc',
        }),
      },
    }

    execSync(buildCommand, buildOptions)
    logSuccess(`Built binary for ${target}`)
    return true
  }
  catch (error) {
    logError(`Failed to build binary for ${target}`)
    logError(`Error: ${error.message}`)
    return false
  }
}

function copyBinaryToPlatformPackage(targetInfo) {
  const { target, platform, binaryName, packageDir } = targetInfo

  try {
    const sourcePath = path.join(
      projectRoot,
      'target',
      target,
      'release',
      binaryName,
    )

    const destDir = path.join(projectRoot, packageDir, 'bin')
    const destPath = path.join(destDir, binaryName)

    if (!fs.existsSync(sourcePath)) {
      logError(`Binary not found: ${sourcePath}`)
      return false
    }

    if (!fs.existsSync(destDir)) {
      fs.mkdirSync(destDir, { recursive: true })
      log(`Created directory: ${destDir}`)
    }

    fs.copyFileSync(sourcePath, destPath)

    if (platform !== 'win32-x64') {
      fs.chmodSync(destPath, 0o755)
    }

    logSuccess(`Copied binary to: ${destPath}`)
    return true
  }
  catch (error) {
    logError(`Failed to copy binary for ${platform}: ${error.message}`)
    return false
  }
}

function validateBinary(targetInfo) {
  const { platform, binaryName, packageDir } = targetInfo
  const binaryPath = path.join(projectRoot, packageDir, 'bin', binaryName)

  try {
    if (!fs.existsSync(binaryPath)) {
      logError(`Binary not found: ${binaryPath}`)
      return false
    }

    if (platform !== 'win32-x64') {
      const stats = fs.statSync(binaryPath)
      if (!(stats.mode & 0o111)) {
        logError(`Binary is not executable: ${binaryPath}`)
        return false
      }
    }

    const stats = fs.statSync(binaryPath)
    const sizeInMB = (stats.size / 1024 / 1024).toFixed(2)

    logSuccess(`Binary validated: ${binaryPath} (${sizeInMB} MB)`)
    return true
  }
  catch (error) {
    logError(`Failed to validate binary for ${platform}: ${error.message}`)
    return false
  }
}

function installLinuxCrossCompiler() {
  if (process.platform !== 'linux') {
    return
  }

  try {
    log('Installing Linux ARM64 cross-compiler...')
    execSync(
      'sudo apt-get update && sudo apt-get install -y gcc-aarch64-linux-gnu',
      {
        stdio: 'pipe',
      },
    )
    logSuccess('Installed Linux ARM64 cross-compiler')
  }
  catch {
    logWarning('Failed to install Linux ARM64 cross-compiler')
    logWarning(
      'You may need to install it manually: sudo apt-get install gcc-aarch64-linux-gnu',
    )
  }
}

async function main() {
  console.warn(
    colors.bold('🔧 Preparing Rari binaries for platform packages\n'),
  )

  let targetsToBuild

  if (currentPlatformOnly) {
    const currentTarget = getCurrentPlatformTarget()
    if (!currentTarget) {
      logError('Unable to determine current platform target')
      logError(
        'Supported platforms: macOS (x64/ARM64), Linux (x64/ARM64), Windows (x64)',
      )
      process.exit(1)
    }

    targetsToBuild = [currentTarget]
    log(
      `Building for current platform only: ${colors.cyan(currentTarget.platform)}`,
    )
    console.warn(
      colors.dim(
        'Use --all flag to build for all platforms (requires cross-compilation tools)',
      ),
    )
  }
  else {
    targetsToBuild = TARGETS
    log('Building for all platforms (cross-compilation mode)')
  }

  console.warn('')

  checkRustInstalled()

  if (buildAll) {
    installLinuxCrossCompiler()
  }

  log('Installing Rust targets...')
  for (const targetInfo of targetsToBuild) {
    installTarget(targetInfo.target)
  }

  console.warn('')

  log('Building binaries...')
  let successCount = 0
  let failureCount = 0

  for (const targetInfo of targetsToBuild) {
    const success = buildBinary(targetInfo.target)
    if (success) {
      successCount++
    }
    else {
      failureCount++
      if (currentPlatformOnly) {
        logError('Failed to build binary for current platform')
        logError('This may indicate a Rust compilation issue')
        process.exit(1)
      }
    }
  }

  console.warn('')

  log('Copying binaries to platform packages...')
  for (const targetInfo of targetsToBuild) {
    if (
      fs.existsSync(
        path.join(
          projectRoot,
          'target',
          targetInfo.target,
          'release',
          targetInfo.binaryName,
        ),
      )
    ) {
      const success = copyBinaryToPlatformPackage(targetInfo)
      if (!success) {
        failureCount++
      }
    }
  }

  console.warn('')

  log('Validating binaries...')
  for (const targetInfo of targetsToBuild) {
    validateBinary(targetInfo)
  }

  console.warn('')

  const totalAttempted = targetsToBuild.length

  if (failureCount === 0) {
    logSuccess(`✨ Successfully prepared ${successCount} platform binaries!`)
    console.warn('')
    console.warn(colors.bold('Platform packages ready:'))
    for (const targetInfo of targetsToBuild) {
      console.warn(
        `  • ${colors.cyan(targetInfo.platform)} → ${targetInfo.packageDir}`,
      )
    }
    console.warn('')
    console.warn(colors.dim('Next steps:'))
    if (currentPlatformOnly) {
      console.warn(colors.dim('  1. Test the binary locally'))
      console.warn(
        colors.dim('  2. Use GitHub Actions for full cross-platform builds'),
      )
      console.warn(
        colors.dim(
          '  3. Or run with --all flag (requires cross-compilation setup)',
        ),
      )
    }
    else {
      console.warn(colors.dim('  1. Test the binaries locally'))
      console.warn(colors.dim('  2. Run the release script: pnpm run release'))
      console.warn(colors.dim('  3. Or publish individual packages'))
    }
  }
  else {
    if (successCount > 0) {
      logWarning(
        `Partial success: ${successCount}/${totalAttempted} binaries built`,
      )
      console.warn('')
      console.warn(colors.bold('Successfully built:'))
      for (const targetInfo of targetsToBuild) {
        const binaryPath = path.join(
          projectRoot,
          'target',
          targetInfo.target,
          'release',
          targetInfo.binaryName,
        )
        if (fs.existsSync(binaryPath)) {
          console.warn(`  • ${colors.green(targetInfo.platform)}`)
        }
      }
    }
    else {
      logError(`Failed to prepare any platform binaries`)
    }

    console.warn('')
    console.warn(colors.bold('Troubleshooting:'))
    if (buildAll) {
      console.warn('  • Cross-compilation requires additional tools:')
      console.warn('    - Linux: Install gcc-*-linux-gnu packages')
      console.warn('    - Windows: Install mingw-w64 toolchain')
      console.warn(
        '    - Use GitHub Actions for reliable cross-platform builds',
      )
      console.warn('  • Or build for current platform only (remove --all flag)')
    }
    else {
      console.warn('  • Ensure Rust is installed: https://rustup.rs/')
      console.warn('  • Check that all required dependencies are installed')
    }

    if (currentPlatformOnly && failureCount > 0) {
      process.exit(1)
    }
  }
}

main().catch((error) => {
  logError(`Unexpected error: ${error.message}`)
  console.error(error)
  process.exit(1)
})
