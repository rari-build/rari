import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import { styleText } from 'node:util'
import {
  createOrBackupConfigFile,
  getInstallCommand,
  logError,
  logInfo,
  logSuccess,
  logWarn,
  printDeploymentSuccess,
  updatePackageJsonForProvider,
} from './utils'

export function createRailwayDeployment() {
  const cwd = process.cwd()

  logInfo('Creating Railway deployment configuration...')

  const addedRari = updatePackageJsonForProvider(cwd, { providerName: 'Railway' })

  const name = getProjectName(cwd)
  const source = getGitHubSource(cwd)
  if (source == null) {
    logWarn(
      'No GitHub remote found. Connect the repository to the service in the Railway dashboard instead.',
    )
  }

  const hasRailpackJson = existsSync(join(cwd, 'railpack.json'))
  createRailwayIac(cwd, { name, source, includeAptPackages: !hasRailpackJson })
  const hasRuntimePackages = hasRailpackJson ? ensureRailpackAptPackages(cwd) : true

  const run = (command: string) => styleText('cyan', command)
  printDeploymentSuccess(
    'Railway',
    [
      ...(hasRailwayDependency(cwd)
        ? []
        : [`${run(getAddDevCommand(cwd, 'railway'))} (provides railway/iac)`]),
      ...(addedRari ? [`${run(getInstallCommand(cwd))} (updates your lockfile for rari)`] : []),
      run('git add .'),
      run('git commit -m "Add Railway deployment"'),
      run(`git push origin ${source?.branch ?? 'main'}`),
      ...(source == null
        ? [
            `Go to ${run('https://railway.app')} and deploy from GitHub`,
            `${run('railway link')} to link this directory to the project`,
            `Check the service name in ${run('.railway/railway.ts')} matches Railway ("${name}")`,
          ]
        : [`${run(`railway init --name ${name}`)} to create and link a Railway project`]),
      `${run('railway config plan')} to preview, then ${run('railway config apply')} to deploy`,
    ],
    [
      ...(hasRuntimePackages ? ['Install libfontconfig1 (required by the Rust runtime)'] : []),
      'Bind to 0.0.0.0 (Railway requirement)',
      "Use Railway's PORT environment variable",
    ],
  )
}

const RUNTIME_APT_PACKAGES = ['libfontconfig1']
const DEFAULT_PROJECT_NAME = 'web'
const GITHUB_REMOTE_REGEX = /github\.com[:/]([^/]+\/[^/]+?)(?:\.git)?\/?$/

interface GitHubSource {
  readonly repo: string
  readonly branch: string | null
}

function readPackageJson(cwd: string): Record<string, unknown> | null {
  try {
    const parsed: unknown = JSON.parse(readFileSync(join(cwd, 'package.json'), 'utf-8'))
    return isRecord(parsed) ? parsed : null
  } catch {
    return null
  }
}

function getProjectName(cwd: string): string {
  const name = readPackageJson(cwd)?.name
  if (typeof name !== 'string' || name === '') return DEFAULT_PROJECT_NAME
  return name.replace(/^@[^/]+\//, '')
}

function hasRailwayDependency(cwd: string): boolean {
  const packageJson = readPackageJson(cwd)
  return ['dependencies', 'devDependencies'].some(field => {
    const deps = packageJson?.[field]
    return isRecord(deps) && 'railway' in deps
  })
}

function getAddDevCommand(cwd: string, pkg: string): string {
  const install = getInstallCommand(cwd)
  if (install === 'npm install') return `npm install -D ${pkg}`
  return `${install.replace(/ install$/, ' add')} -D ${pkg}`
}

function git(cwd: string, args: readonly string[]): string | null {
  try {
    const output = execFileSync('git', args, {
      cwd,
      encoding: 'utf-8',
      stdio: ['ignore', 'pipe', 'ignore'],
    })
    return output.trim() || null
  } catch {
    return null
  }
}

function getGitHubSource(cwd: string): GitHubSource | null {
  const remote = git(cwd, ['remote', 'get-url', 'origin'])
  const repo = remote == null ? null : GITHUB_REMOTE_REGEX.exec(remote)?.[1]
  if (repo == null) return null
  return { repo, branch: git(cwd, ['symbolic-ref', '--short', 'HEAD']) }
}

function quote(value: string): string {
  return `'${value.replaceAll('\\', '\\\\').replaceAll("'", "\\'")}'`
}

interface RailwayIacOptions {
  readonly name: string
  readonly source: GitHubSource | null
  readonly includeAptPackages: boolean
}

function createRailwayIac(cwd: string, { name, source, includeAptPackages }: RailwayIacOptions) {
  const imports = ['defineRailway', ...(source == null ? [] : ['github']), 'project', 'service']

  const sourceLine =
    source == null
      ? ''
      : `
    source: github(${quote(source.repo)}${source.branch == null ? '' : `, { branch: ${quote(source.branch)} }`}),`

  const env = includeAptPackages
    ? `
    env: {
      RAILPACK_DEPLOY_APT_PACKAGES: '... ${RUNTIME_APT_PACKAGES.join(' ')}',
    },`
    : ''

  const railwayConfig = `import { ${imports.join(', ')} } from 'railway/iac'

export default defineRailway(() => {
  const app = service(${quote(name)}, {${sourceLine}
    start: 'npm start',
    healthcheck: '/_rari/health',
    healthcheckTimeout: 300,
    deploy: {
      restartPolicyType: 'ON_FAILURE',
      restartPolicyMaxRetries: 3,
    },${env}
  })

  return project(${quote(name)}, { resources: [app] })
})
`

  mkdirSync(join(cwd, '.railway'), { recursive: true })
  createOrBackupConfigFile(cwd, '.railway/railway.ts', railwayConfig)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function ensureRailpackAptPackages(cwd: string): boolean {
  const railpackPath = join(cwd, 'railpack.json')

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
