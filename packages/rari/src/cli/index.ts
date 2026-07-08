import type { ChildProcess, SpawnOptions } from 'node:child_process'
import { spawn } from 'node:child_process'
import { existsSync, readdirSync, readFileSync, realpathSync, rmSync } from 'node:fs'
import { resolve } from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import { parseArgs, styleText } from 'node:util'
import { logError, logInfo, logSuccess, logWarn } from '@rari/logger'
import { getBinaryPath, getInstallationInstructions } from './platform'

type PackageManager = 'pnpm' | 'yarn' | 'bun' | 'npm'

interface ProjectContext {
  cwd: string
  packageManager: PackageManager
  viteBin: 'vp' | 'vite'
}

let cachedProjectContext: ProjectContext | null = null

function detectPackageManagerFromDir(dir: string): PackageManager | null {
  const entries = existsSync(dir) ? new Set(readDirNames(dir)) : new Set<string>()

  if (entries.has('pnpm-lock.yaml'))
    return 'pnpm'
  if (entries.has('yarn.lock'))
    return 'yarn'
  if (entries.has('bun.lockb') || entries.has('bun.lock'))
    return 'bun'
  if (entries.has('package-lock.json'))
    return 'npm'

  return readPackageManagerField(dir)
}

function readDirNames(dir: string): string[] {
  try {
    return readdirSync(dir)
  }
  catch {
    return []
  }
}

function readPackageManagerField(dir: string): PackageManager | null {
  try {
    const pkgPath = resolve(dir, 'package.json')
    if (!existsSync(pkgPath))
      return null

    const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8')) as { packageManager?: string }
    if (pkg.packageManager?.startsWith('pnpm'))
      return 'pnpm'
    if (pkg.packageManager?.startsWith('yarn'))
      return 'yarn'
    if (pkg.packageManager?.startsWith('bun'))
      return 'bun'
    if (pkg.packageManager?.startsWith('npm'))
      return 'npm'
  }
  catch {}

  return null
}

function detectViteBinFromDir(dir: string): 'vp' | 'vite' | null {
  try {
    const pkgPath = resolve(dir, 'package.json')
    if (!existsSync(pkgPath))
      return null

    const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8')) as {
      dependencies?: Record<string, string>
      devDependencies?: Record<string, string>
    }
    const deps = { ...pkg.dependencies, ...pkg.devDependencies }
    if (deps['vite-plus'])
      return 'vp'
    if (deps.vite)
      return 'vite'
  }
  catch {}

  return null
}

function discoverProjectContext(cwd: string): ProjectContext {
  let currentDir = cwd
  let packageManager: PackageManager | null = null
  let viteBin: 'vp' | 'vite' | null = null

  while (true) {
    if (!packageManager)
      packageManager = detectPackageManagerFromDir(currentDir)

    if (!viteBin)
      viteBin = detectViteBinFromDir(currentDir)

    if (packageManager && viteBin)
      break

    const parentDir = resolve(currentDir, '..')
    if (parentDir === currentDir)
      break

    currentDir = parentDir
  }

  return {
    cwd,
    packageManager: packageManager ?? 'npm',
    viteBin: viteBin ?? 'vite',
  }
}

function getProjectContext(): ProjectContext {
  const cwd = process.cwd()
  if (cachedProjectContext?.cwd === cwd)
    return cachedProjectContext

  cachedProjectContext = discoverProjectContext(cwd)
  return cachedProjectContext
}

function detectPackageManager(): PackageManager {
  return getProjectContext().packageManager
}

function getPackageExecutor(): string {
  const pm = getProjectContext().packageManager
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

function resolveLocalBin(binName: string, startDir = process.cwd()): string | null {
  const isWindows = process.platform === 'win32'
  let currentDir = startDir

  while (true) {
    const binDir = resolve(currentDir, 'node_modules', '.bin')
    const candidates = isWindows
      ? [
          resolve(binDir, `${binName}.cmd`),
          resolve(binDir, `${binName}.exe`),
          resolve(binDir, binName),
        ]
      : [resolve(binDir, binName)]

    for (const candidate of candidates) {
      if (existsSync(candidate))
        return candidate
    }

    const parentDir = resolve(currentDir, '..')
    if (parentDir === currentDir)
      break

    currentDir = parentDir
  }

  return null
}

function crossPlatformSpawn(command: string, args: string[], options: SpawnOptions = {}) {
  const isWindows = process.platform === 'win32'

  if (command === 'npx') {
    const executor = getPackageExecutor()
    if (executor.includes('bun')) {
      const bunxCommand = isWindows ? 'bunx.cmd' : 'bunx'
      return spawn(bunxCommand, args, { ...options, shell: isWindows })
    }
    if (executor.includes('pnpm'))
      return spawn(executor, ['exec', ...args], { ...options, shell: isWindows })
    if (executor.includes('yarn')) {
      const [bin, ...rest] = args
      const yarnArgs = bin === 'vp'
        ? ['dlx', '-p', 'vite-plus', 'vp', ...rest]
        : ['dlx', ...args]
      return spawn(executor, yarnArgs, { ...options, shell: isWindows })
    }
  }

  if (isWindows && command === 'npx')
    return spawn('npx.cmd', args, { ...options, shell: true })

  return spawn(command, args, options)
}

function spawnTool(binName: string, args: string[], options: SpawnOptions = {}) {
  const localBin = resolveLocalBin(binName)
  if (localBin) {
    const needsShell = process.platform === 'win32' && localBin.endsWith('.cmd')
    return spawn(localBin, args, needsShell ? { ...options, shell: true } : options)
  }

  return crossPlatformSpawn('npx', [binName, ...args], options)
}

async function waitForProcess(
  child: ChildProcess,
  options: { tolerateErrors?: boolean } = {},
): Promise<number | null> {
  return new Promise((resolve, reject) => {
    child.on('exit', (code) => {
      resolve(code)
    })
    child.on('error', (error) => {
      if (options.tolerateErrors) {
        logWarn(normalizeError(error))
        resolve(1)
      }
      else {
        reject(error)
      }
    })
  })
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

function loadProjectEnv() {
  const envPath = resolve(process.cwd(), '.env')
  if (existsSync(envPath))
    process.loadEnvFile(envPath)
}

function parseCliArgv(argv: string[]) {
  const { values, positionals } = parseArgs({
    args: argv,
    options: {
      help: { type: 'boolean', short: 'h', default: false },
    },
    allowPositionals: true,
    strict: false,
  })

  if (values.help)
    return { command: 'help', args: [] as string[] }

  const [command, ...args] = positionals
  return { command, args }
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
  await cleanDistFolder()
  const viteBin = getProjectContext().viteBin

  logInfo('Type checking...')
  const typecheckProcess = spawnTool('tsc', [], {
    stdio: 'inherit',
    cwd: process.cwd(),
  })

  const typecheckCode = await waitForProcess(typecheckProcess)
  if (typecheckCode === 0) {
    logSuccess('Type check passed')
  }
  else {
    logError(`Type check failed with code ${typecheckCode}`)
    throw new Error(`Type check failed with code ${typecheckCode}`)
  }

  logInfo('Building for production...')
  const buildProcess = spawnTool(viteBin, ['build'], {
    stdio: 'inherit',
    cwd: process.cwd(),
  })

  const buildCode = await waitForProcess(buildProcess)
  if (buildCode === 0) {
    logSuccess('Build complete')
  }
  else {
    logError(`Build failed with code ${buildCode}`)
    throw new Error(`Build failed with code ${buildCode}`)
  }

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

    const code = await waitForProcess(optimizeProcess, { tolerateErrors: true })
    if (code !== 0)
      logWarn(`Image pre-optimization exited with code ${code}`)
  }
  catch (error) {
    logWarn(`Could not pre-optimize images: ${normalizeError(error)}`)
  }
}

async function runViteDev() {
  const viteBin = getProjectContext().viteBin
  const distPath = resolve(process.cwd(), 'dist')

  if (!existsSync(distPath)) {
    logInfo('First run detected - building project...')

    const buildProcess = spawnTool(viteBin, ['build', '--mode', 'development'], {
      stdio: 'inherit',
      cwd: process.cwd(),
    })

    const buildCode = await waitForProcess(buildProcess)
    if (buildCode === 0) {
      logSuccess('Initial build complete')
    }
    else {
      logError(`Build failed with code ${buildCode}`)
      throw new Error(`Build failed with code ${buildCode}`)
    }
  }

  logInfo(`Starting Vite${viteBin === 'vp' ? '+' : ''} dev server...`)
  const viteProcess = spawnTool(viteBin, ['dev'], {
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

  viteProcess.on('exit', (code: number | null) => {
    if (code !== 0 && code !== null) {
      logError(`Vite exited with code ${code}`)
      process.exit(code)
    }
  })

  return new Promise(() => {})
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
    // Keep the Rust server out of the terminal's process group so Ctrl+C
    // only reaches this Node process. Deno intercepts SIGINT and won't shut
    // down cleanly, but it does handle SIGTERM.
    detached: process.platform !== 'win32',
    env: {
      ...process.env,
      RUST_LOG: process.env.RUST_LOG || 'error',
    },
  })

  let shuttingDown = false
  let forceKillTimer: NodeJS.Timeout | undefined

  const shutdown = () => {
    if (shuttingDown)
      return
    shuttingDown = true
    logInfo('shutting down...')
    rustServer.kill('SIGTERM')
    forceKillTimer = setTimeout(() => {
      rustServer.kill('SIGKILL')
      process.exit(1)
    }, 5000)
    forceKillTimer.unref?.()
  }

  process.on('SIGINT', shutdown)
  process.on('SIGTERM', shutdown)

  rustServer.on('error', (error: Error) => {
    logError(`Failed to start rari server: ${error.message}`)
    if (error.message.includes('ENOENT'))
      logError('Binary not found. Please ensure rari is properly installed.')
    process.exit(1)
  })

  rustServer.on('exit', (code: number | null, signal: string | null) => {
    if (forceKillTimer)
      clearTimeout(forceKillTimer)

    if (signal) {
      if (shuttingDown) {
        logInfo(`server stopped by signal ${signal}`)
        process.exit(0)
      }
      logError(`server stopped unexpectedly by signal ${signal}`)
      process.exit(1)
    }
    else if (code === 0) {
      logSuccess('server stopped successfully')
      process.exit(0)
    }
    else {
      logError(`server exited with code ${code}`)
      process.exit(code || 1)
    }
  })

  return new Promise(() => {})
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

async function cleanDistFolder() {
  const distPath = resolve(process.cwd(), 'dist')

  if (existsSync(distPath)) {
    logInfo('Cleaning dist folder...')
    rmSync(distPath, { recursive: true, force: true })
    logSuccess('Cleaned dist folder')
  }
  else {
    logInfo('No dist folder to clean')
  }
}

function printHelp() {
  console.warn(`${styleText('bold', 'rari CLI')}

${styleText('bold', 'Usage:')}
  ${styleText('cyan', 'rari dev')}                 Start the development server with Vite
  ${styleText('cyan', 'rari build')}               Build for production
  ${styleText('cyan', 'rari start')}               Start the rari server (defaults to production)
  ${styleText('cyan', 'rari clean')}               Remove the dist folder
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

  ${styleText('gray', '# Clean dist folder')}
  ${styleText('cyan', 'rari clean')}

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
}

async function main() {
  loadProjectEnv()

  const { command, args } = parseCliArgv(process.argv.slice(2))

  switch (command) {
    case undefined:
    case 'help':
      printHelp()
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

    case 'clean':
      await cleanDistFolder()
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

function isCliMainModule(): boolean {
  const invokedPath = process.argv[1] ? realpathSync(resolve(process.argv[1])) : ''
  const currentPath = realpathSync(fileURLToPath(import.meta.url))
  return Boolean(invokedPath) && currentPath === invokedPath
}

if (isCliMainModule()) {
  main().catch((error) => {
    logError(`CLI Error: ${normalizeError(error)}`)
    console.error(error)
    process.exit(1)
  })
}

export { detectPackageManager, getDeploymentConfig, isRailwayEnvironment, isRenderEnvironment }
