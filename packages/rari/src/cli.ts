#!/usr/bin/env node

import { spawn } from 'node:child_process'
import process from 'node:process'
import colors from 'picocolors'
import { getBinaryPath, getInstallationInstructions } from './platform'

const [, , command, ...args] = process.argv

function logInfo(message: string) {
  console.warn(`${colors.blue('info')} ${message}`)
}

function logSuccess(message: string) {
  console.warn(`${colors.green('✓')} ${message}`)
}

function logError(message: string) {
  console.error(`${colors.red('✗')} ${message}`)
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
  const mode = process.env.NODE_ENV === 'production' ? 'production' : 'development'
  const host = isPlatformEnvironment() ? '0.0.0.0' : '127.0.0.1'

  return { port, mode, host }
}

async function startRustServer(): Promise<void> {
  let binaryPath: string

  try {
    binaryPath = getBinaryPath()
  }
  catch {
    logError('Failed to obtain Rari binary')
    logError(getInstallationInstructions())
    process.exit(1)
  }

  const { port, mode, host } = getDeploymentConfig()

  if (isPlatformEnvironment()) {
    const platformName = getPlatformName()
    logInfo(`${platformName} environment detected`)
    logInfo(`Starting Rari server for ${platformName} deployment...`)
    logInfo(`Mode: ${mode}, Host: ${host}, Port: ${port}`)
  }
  else {
    logInfo(`starting Rari server in ${mode} mode on port ${port}...`)
  }

  logInfo(`using binary: ${binaryPath}`)

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
    logError(`Failed to start Rari server: ${error.message}`)
    if (error.message.includes('ENOENT')) {
      logError('Binary not found. Please ensure Rari is properly installed.')
    }
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

  const { createRailwayDeployment } = await import('./deployment/railway.js')
  await createRailwayDeployment()
}

async function deployToRender() {
  logInfo('Setting up Render deployment...')

  if (isPlatformEnvironment()) {
    logError(`Already running in ${getPlatformName()} environment. Use "rari start" instead.`)
    process.exit(1)
  }

  const { createRenderDeployment } = await import('./deployment/render.js')
  await createRenderDeployment()
}

async function main() {
  switch (command) {
    case undefined:
    case 'help':
    case '--help':
    case '-h':
      console.warn(`${colors.bold('Rari CLI')}

${colors.bold('Usage:')}
  ${colors.cyan('rari start')}              Start the Rari server
  ${colors.cyan('rari deploy railway')}     Setup Railway deployment
  ${colors.cyan('rari deploy render')}      Setup Render deployment
  ${colors.cyan('rari help')}               Show this help message

${colors.bold('Environment Variables:')}
  ${colors.yellow('PORT')}                    Server port (default: 3000)
  ${colors.yellow('RSC_PORT')}               Alternative server port
  ${colors.yellow('NODE_ENV')}               Environment (development/production)
  ${colors.yellow('RUST_LOG')}               Rust logging level (default: info)

${colors.bold('Examples:')}
  ${colors.gray('# Start development server on port 3000')}
  ${colors.cyan('rari start')}

  ${colors.gray('# Start production server on port 8080')}
  ${colors.cyan('PORT=8080 NODE_ENV=production rari start')}

  ${colors.gray('# Setup Railway deployment')}
  ${colors.cyan('rari deploy railway')}

  ${colors.gray('# Setup Render deployment')}
  ${colors.cyan('rari deploy render')}

  ${colors.gray('# Start with debug logging')}
  ${colors.cyan('RUST_LOG=debug rari start')}

${colors.bold('Deployment:')}
  ${colors.cyan('rari deploy railway')}     Creates Railway deployment files
  ${colors.cyan('rari deploy render')}      Creates Render deployment files

  Platform deployment automatically detects the environment and configures:
  - Host binding (0.0.0.0 for platforms, 127.0.0.1 for local)
  - Port from platform's PORT environment variable
  - Production mode optimization

${colors.bold('Binary Resolution:')}
  1. Platform-specific package (rari-{platform}-{arch})
  2. Global binary in PATH
  3. Install from source with Cargo

${colors.bold('Notes:')}
  - Platform binary is automatically detected and used
  - Platform deployment is automatically detected and configured
  - Use Ctrl+C to stop the server gracefully

`)
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
      console.error(`${colors.bold('Unknown command:')} ${command}`)
      console.warn(`Run "${colors.cyan('rari help')}" for available commands`)
      process.exit(1)
  }
}

main().catch((error) => {
  logError(`CLI Error: ${error.message}`)
  console.error(error)
  process.exit(1)
})
