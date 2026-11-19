import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import { styleText } from 'node:util'
import {
  createOrBackupConfigFile,
  logError,
  logInfo,
  logSuccess,
  printDeploymentSuccess,
  updatePackageJsonForProvider,
} from './utils'

export function createRailwayDeployment() {
  const cwd = process.cwd()

  logInfo('Creating Railway deployment configuration...')

  updatePackageJsonForProvider(cwd, { providerName: 'Railway' })

  createRailwayToml(cwd)
  const hasRuntimePackages = ensureRailpackAptPackages(cwd)

  printDeploymentSuccess(
    'Railway',
    [
      styleText('cyan', 'git add .'),
      styleText('cyan', 'git commit -m "Add Railway deployment"'),
      styleText('cyan', 'git push origin main'),
      `Go to ${styleText('cyan', 'https://railway.app')} and deploy from GitHub`,
    ],
    [
      ...(hasRuntimePackages ? ['Install libfontconfig1 (required by the Rust runtime)'] : []),
      'Bind to 0.0.0.0 (Railway requirement)',
      "Use Railway's PORT environment variable",
    ],
  )
}

function createRailwayToml(cwd: string) {
  const railwayConfig = `[build]
builder = "RAILPACK"

[deploy]
startCommand = "npm start"
healthcheckPath = "/_rari/health"
healthcheckTimeout = 300
restartPolicyType = "ON_FAILURE"
restartPolicyMaxRetries = 3
`

  createOrBackupConfigFile(cwd, 'railway.toml', railwayConfig)
}

const RAILPACK_SCHEMA = 'https://schema.railpack.com'
const RUNTIME_APT_PACKAGES = ['libfontconfig1']

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function ensureRailpackAptPackages(cwd: string): boolean {
  const railpackPath = join(cwd, 'railpack.json')

  if (!existsSync(railpackPath)) {
    const config = {
      $schema: RAILPACK_SCHEMA,
      deploy: { aptPackages: ['...', ...RUNTIME_APT_PACKAGES] },
    }
    writeFileSync(railpackPath, `${JSON.stringify(config, null, 2)}\n`)
    logSuccess('Created railpack.json configuration')
    return true
  }

  let config: unknown
  try {
    config = JSON.parse(readFileSync(railpackPath, 'utf-8'))
  } catch {
    config = null
  }

  if (!isRecord(config) || (config.deploy !== undefined && !isRecord(config.deploy))) {
    logError(
      `Could not update railpack.json. Add ${RUNTIME_APT_PACKAGES.join(', ')} to deploy.aptPackages manually.`,
    )
    return false
  }

  const deploy = isRecord(config.deploy) ? config.deploy : {}
  const existing = Array.isArray(deploy.aptPackages)
    ? deploy.aptPackages.filter((pkg): pkg is string => typeof pkg === 'string')
    : ['...']
  const missing = RUNTIME_APT_PACKAGES.filter(pkg => !existing.includes(pkg))
  if (missing.length === 0) return true

  config.deploy = { ...deploy, aptPackages: [...existing, ...missing] }
  writeFileSync(railpackPath, `${JSON.stringify(config, null, 2)}\n`)
  logSuccess(`Added ${missing.join(', ')} to railpack.json deploy.aptPackages`)
  return true
}
