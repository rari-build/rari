#!/usr/bin/env node

import type { SpawnOptions } from 'node:child_process'
import { spawn } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import process from 'node:process'
import { styleText } from 'node:util'
import { logError, logInfo, logSuccess, logWarn } from '@rari/logger'
import { getBinaryPath, getInstallationInstructions } from './platform'

function loadEnvFile() {
  const envPath = resolve(process.cwd(), '.env')
  if (existsSync(envPath)) {
    const envContent = readFileSync(envPath, 'utf-8')
    for (const line of envContent.split('\n')) {
      const trimmed = line.trim()

      if (!trimmed || trimmed.startsWith('#'))
        continue

      const match = trimmed.match(/^([^=]+)=(.*)$/)
      if (match) {
        const [, key, value] = match
        const cleanKey = key.trim()
        let cleanValue = value.trim()

        if ((cleanValue.startsWith('"') && cleanValue.endsWith('"'))
          || (cleanValue.startsWith('\'') && cleanValue.endsWith('\''))) { cleanValue = cleanValue.slice(1, -1) }

        if (!process.env[cleanKey])
          process.env[cleanKey] = cleanValue
      }
    }
  }
}

loadEnvFile()

const [, , command, ...args] = process.argv

function detectPackageManager(): 'pnpm' | 'yarn' | 'bun' | 'npm' {
  let currentDir = process.cwd()
  const root = resolve('/')
  let iterations = 0
  const maxIterations = 20

  while (currentDir !== root && iterations < maxIterations) {
    iterations++

    if (existsSync(resolve(currentDir, 'pnpm-lock.yaml')))
      return 'pnpm'
    if (existsSync(resolve(currentDir, 'yarn.lock')))
      return 'yarn'
    if (existsSync(resolve(currentDir, 'bun.lockb')))
      return 'bun'
    if (existsSync(resolve(currentDir, 'package-lock.json')))
      return 'npm'

    try {
      const pkgPath = resolve(currentDir, 'package.json')
      if (existsSync(pkgPath)) {
        const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8'))
        if (pkg.packageManager?.startsWith('pnpm'))
          return 'pnpm'
        if (pkg.packageManager?.startsWith('yarn'))
          return 'yarn'
        if (pkg.packageManager?.startsWith('bun'))
          return 'bun'
        if (pkg.packageManager?.startsWith('npm'))
          return 'npm'
      }
    }
    catch {}

    const parentDir = resolve(currentDir, '..')
    if (parentDir === currentDir)
      break
    currentDir = parentDir
  }

  return 'npm'
}

function getPackageExecutor(): string {
  const pm = detectPackageManager()
  const isWindows = process.platform === 'win32'

  switch (pm) {
    case 'bun':
      return isWindows ? 'bun.cmd' : 'bun'
    case 'pnpm':
      return isWindows ? 'pnpm.cmd' : 'pnpm'
    case 'yarn':
      return isWindows ? 'yarn.cmd' : 'yarn'
    default:
      return isWindows ? 'npx.cmd' : 'npx'
  }
}

function crossPlatformSpawn(command: string, args: string[], options: SpawnOptions = {}) {
  if (command === 'npx') {
    const executor = getPackageExecutor()
    if (executor.includes('bun'))
      return spawn(executor, ['x', ...args], options)
    if (executor.includes('pnpm'))
      return spawn(executor, ['exec', ...args], options)
    if (executor.includes('yarn'))
      return spawn(executor, ['dlx', ...args], options)
  }

  const isWindows = process.platform === 'win32'
  if (isWindows && command === 'npx')
    return spawn('npx.cmd', args, options)

  return spawn(command, args, options)
}

function normalizeError(error: unknown): string {
  if (error instanceof Error)
    return error.message
  if (typeof error === 'string')
    return error
  try {
    return JSON.stringify(error)
  }
  catch {
    return String(error)
  }
}

function isRailwayEnvironment(): boolean {
  return !!(
    process.env.RAILWAY_ENVIRONMENT
    || process.env.RAILWAY_PROJECT_ID
    || process.env.RAILWAY_SERVICE_ID
  )
}

function isRenderEnvironment(): boolean {
  return !!(
    process.env.RENDER
    || process.env.RENDER_SERVICE_ID
    || process.env.RENDER_SERVICE_NAME
  )
}

function isPlatformEnvironment(): boolean {
  return isRailwayEnvironment() || isRenderEnvironment()
}

function getPlatformName(): string {
  if (isRailwayEnvironment())
    return 'Railway'
  if (isRenderEnvironment())
    return 'Render'

  return 'local'
}

function getDeploymentConfig() {
  const port = process.env.PORT || process.env.RSC_PORT || '3000'
  const mode = process.env.NODE_ENV || 'production'
  const host = isPlatformEnvironment() ? '0.0.0.0' : '127.0.0.1'

  return { port, mode, host }
}

async function runViteBuild() {
  const { existsSync, rmSync } = await import('node:fs')
  const { resolve } = await import('node:path')

  const distPath = resolve(process.cwd(), 'dist')

  if (existsSync(distPath)) {
    logInfo('Cleaning dist folder...')
    rmSync(distPath, { recursive: true, force: true })
  }

  logInfo('Type checking...')
  const typecheckProcess = crossPlatformSpawn('npx', ['tsgo'], {
    stdio: 'inherit',
    cwd: process.cwd(),
  })

  await new Promise<void>((resolve, reject) => {
    typecheckProcess.on('exit', (code) => {
      if (code === 0) {
        logSuccess('Type check passed')
        resolve()
      }
      else {
        logError(`Type check failed with code ${code}`)
        reject(new Error(`Type check failed with code ${code}`))
      }
    })
    typecheckProcess.on('error', reject)
  })

  logInfo('Building for production...')
  const buildProcess = crossPlatformSpawn('npx', ['vite', 'build'], {
    stdio: 'inherit',
    cwd: process.cwd(),
  })

  await new Promise<void>((resolve, reject) => {
    buildProcess.on('exit', (code) => {
      if (code === 0) {
        logSuccess('Build complete')
        resolve()
      }
      else {
        logError(`Build failed with code ${code}`)
        reject(new Error(`Build failed with code ${code}`))
      }
    })
    buildProcess.on('error', reject)
  })

  await preOptimizeImages()
}

async function preOptimizeImages() {
  const imageConfigPath = resolve(process.cwd(), 'dist', 'server', 'image.json')

  if (!existsSync(imageConfigPath))
    return

  const publicPath = resolve(process.cwd(), 'public')

  if (!existsSync(publicPath))
    return

  try {
    const binaryPath = getBinaryPath()

    const optimizeProcess = spawn(binaryPath, ['optimize-images'], {
      stdio: 'inherit',
      cwd: process.cwd(),
      shell: false,
    })

    await new Promise<void>((resolve) => {
      optimizeProcess.on('exit', (code) => {
        if (code === 0) {
          resolve()
        }
        else {
          logWarn(`Image pre-optimization exited with code ${code}`)
          resolve()
        }
      })
      optimizeProcess.on('error', (err) => {
        logWarn(`Image pre-optimization error: ${normalizeError(err)}`)
        resolve()
      })
    })
  }
  catch (error) {
    logWarn(`Could not pre-optimize images: ${normalizeError(error)}`)
  }
}

async function runViteDev() {
  const { existsSync } = await import('node:fs')
  const { resolve } = await import('node:path')

  const distPath = resolve(process.cwd(), 'dist')

  if (!existsSync(distPath)) {
    logInfo('First run detected - building project...')

    const buildProcess = crossPlatformSpawn('npx', ['vite', 'build', '--mode', 'development'], {
      stdio: 'inherit',
      cwd: process.cwd(),
    })

    await new Promise<void>((resolve, reject) => {
      buildProcess.on('exit', (code) => {
        if (code === 0) {
          logSuccess('Initial build complete')
          resolve()
        }
        else {
          logError(`Build failed with code ${code}`)
          reject(new Error(`Build failed with code ${code}`))
        }
      })
      buildProcess.on('error', reject)
    })
  }

  logInfo('Starting Vite dev server...')
  const viteProcess = crossPlatformSpawn('npx', ['vite'], {
    stdio: 'inherit',
    cwd: process.cwd(),
  })

  const shutdown = () => {
    logInfo('Shutting down dev server...')
    viteProcess.kill('SIGTERM')
  }

  process.on('SIGINT', shutdown)
  process.on('SIGTERM', shutdown)

  viteProcess.on('error', (error: Error) => {
    logError(`Failed to start Vite: ${error.message}`)
    process.exit(1)
  })

  viteProcess.on('exit', (code: number) => {
    if (code !== 0 && code !== null) {
      logError(`Vite exited with code ${code}`)
      process.exit(code)
    }
  })

  return new Promise(() => { })
}

async function startRustServer(): Promise<void> {
  let binaryPath: string

  try {
    binaryPath = getBinaryPath()
  }
  catch {
    logError('Failed to obtain rari binary')
    logError(getInstallationInstructions())
    process.exit(1)
  }

  const { port, mode, host } = getDeploymentConfig()

  if (isPlatformEnvironment()) {
    const platformName = getPlatformName()
    logInfo(`${platformName} environment detected`)
    logInfo(`Starting rari server for ${platformName} deployment...`)
    logInfo(`Mode: ${mode}, Host: ${host}, Port: ${port}`)
    logInfo(`using binary: ${binaryPath}`)
  }

  const args = ['--mode', mode, '--port', port, '--host', host]

  const rustServer = spawn(binaryPath, args, {
    stdio: 'inherit',
    cwd: process.cwd(),
    env: {
      ...process.env,
      RUST_LOG: process.env.RUST_LOG || 'error',
    },
  })

  const shutdown = () => {
    logInfo('shutting down...')
    rustServer.kill('SIGTERM')
  }

  process.on('SIGINT', shutdown)
  process.on('SIGTERM', shutdown)

  rustServer.on('error', (error: Error) => {
    logError(`Failed to start rari server: ${error.message}`)
    if (error.message.includes('ENOENT'))
      logError('Binary not found. Please ensure rari is properly installed.')
    process.exit(1)
  })

  rustServer.on('exit', (code: number, signal: string) => {
    if (signal) {
      logInfo(`server stopped by signal ${signal}`)
    }
    else if (code === 0) {
      logSuccess('server stopped successfully')
    }
    else {
      logError(`server exited with code ${code}`)
      process.exit(code || 1)
    }
  })

  return new Promise(() => { })
}

async function deployToRailway() {
  logInfo('Setting up Railway deployment...')

  if (isPlatformEnvironment()) {
    logError(`Already running in ${getPlatformName()} environment. Use "rari start" instead.`)
    process.exit(1)
  }

  const { createRailwayDeployment } = await import('@rari/deploy/railway')
  await createRailwayDeployment()
}

async function deployToRender() {
  logInfo('Setting up Render deployment...')

  if (isPlatformEnvironment()) {
    logError(`Already running in ${getPlatformName()} environment. Use "rari start" instead.`)
    process.exit(1)
  }

  const { createRenderDeployment } = await import('@rari/deploy/render')
  await createRenderDeployment()
}

async function main() {
  switch (command) {
    case undefined:
    case 'help':
    case '--help':
    case '-h':
      console.warn(`${styleText('bold', 'rari CLI')}

${styleText('bold', 'Usage:')}
  ${styleText('cyan', 'rari dev')}                 Start the development server with Vite
  ${styleText('cyan', 'rari build')}               Build for production
  ${styleText('cyan', 'rari start')}               Start the rari server (defaults to production)
  ${styleText('cyan', 'rari deploy railway')}      Setup Railway deployment
  ${styleText('cyan', 'rari deploy render')}       Setup Render deployment
  ${styleText('cyan', 'rari help')}                Show this help message

${styleText('bold', 'Environment Variables:')}
  ${styleText('yellow', 'PORT')}                     Server port (default: 3000)
  ${styleText('yellow', 'RSC_PORT')}                 Alternative server port
  ${styleText('yellow', 'NODE_ENV')}                 Environment (default: production for start, development for dev)
  ${styleText('yellow', 'RUST_LOG')}                 Rust logging level (default: info)

${styleText('bold', 'Examples:')}
  ${styleText('gray', '# Start development server with Vite')}
  ${styleText('cyan', 'rari dev')}

  ${styleText('gray', '# Build for production')}
  ${styleText('cyan', 'rari build')}

  ${styleText('gray', '# Start production server (default)')}
  ${styleText('cyan', 'rari start')}

  ${styleText('gray', '# Start in development mode')}
  ${styleText('cyan', 'NODE_ENV=development rari start')}

  ${styleText('gray', '# Start production server on port 8080')}
  ${styleText('cyan', 'PORT=8080 rari start')}

  ${styleText('gray', '# Setup Railway deployment')}
  ${styleText('cyan', 'rari deploy railway')}

  ${styleText('gray', '# Setup Render deployment')}
  ${styleText('cyan', 'rari deploy render')}

  ${styleText('gray', '# Start with debug logging')}
  ${styleText('cyan', 'RUST_LOG=debug rari start')}

${styleText('bold', 'Deployment:')}
  ${styleText('cyan', 'rari deploy railway')}     Creates Railway deployment files
  ${styleText('cyan', 'rari deploy render')}      Creates Render deployment files

  Platform deployment automatically detects the environment and configures:
  - Host binding (0.0.0.0 for platforms, 127.0.0.1 for local)
  - Port from platform's PORT environment variable
  - Production mode optimization

${styleText('bold', 'Binary Resolution:')}
  1. Platform-specific package (rari-{platform}-{arch})
  2. Global binary in PATH
  3. Install from source with Cargo

${styleText('bold', 'Notes:')}
  - 'rari start' defaults to production mode unless NODE_ENV is set
  - 'rari dev' runs in development mode with Vite hot reload
  - 'rari build' cleans, type checks, and builds for production
  - Platform binary is automatically detected and used
  - Platform deployment is automatically detected and configured
  - Use Ctrl+C to stop the server gracefully

`)
      break

    case 'dev':
      await runViteDev()
      break

    case 'build':
      await runViteBuild()
      break

    case 'start':
      await startRustServer()
      break

    case 'deploy':
      if (args[0] === 'railway') {
        await deployToRailway()
      }
      else if (args[0] === 'render') {
        await deployToRender()
      }
      else {
        logError('Unknown deployment target. Available: railway, render')
        process.exit(1)
      }
      break

    default:
      console.error(`${styleText('bold', 'Unknown command:')} ${command}`)
      console.warn(`Run "${styleText('cyan', 'rari help')}" for available commands`)
      process.exit(1)
  }
}

const isMainModule = process.argv[1] && (
  import.meta.url === `file://${process.argv[1]}`
    || (import.meta.url.endsWith('/dist/cli.mjs') && process.argv[1].includes('cli.mjs'))
)

if (isMainModule) {
  main().catch((error) => {
    logError(`CLI Error: ${error.message}`)
    console.error(error)
    process.exit(1)
  })
}
