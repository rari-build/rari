import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import { logError, logInfo, logSuccess, logWarn } from '@rari/logger'

export { logError, logInfo, logSuccess, logWarn }

export const MIN_SUPPORTED_NODE_MAJOR = 22

const AND_SPLIT_REGEX = /\s+(?:&&\s+)?/
const LOWER_BOUND_REGEX = /^>=?\s*(\d+)/
const UPPER_BOUND_REGEX = /^<=?\s*(\d+)/
const UPPER_BOUND_ONLY_REGEX = /^<=?\s*\d+/
const SEMVER_RANGE_REGEX = /^>=?\s*(\d+)\.(\d+)\.(\d+)/
const EXACT_SEMVER_REGEX = /^=?\s*(\d+)\.(\d+)\.(\d+)/
const CARET_RANGE_REGEX = /^\^\s*(\d+)\.(\d+)\.(\d+)/
const TILDE_RANGE_REGEX = /^~\s*(\d+)\.(\d+)\.(\d+)/
const MAJOR_MINOR_REGEX = /^(?:>=?|<=?|[=~^])?\s*(\d+)\.(\d+)/
const WILDCARD_REGEX = /^(\d+)\.(?:x|\*)/i
const MAJOR_ONLY_REGEX = /^(?:>=?|[=~^])\s*(\d+)(?:\s|$)/
const NUMBER_ONLY_REGEX = /^(\d+)$/

export function isNodeVersionSufficient(
  versionRange: string,
  minMajor: number = MIN_SUPPORTED_NODE_MAJOR,
): boolean {
  const cleaned = versionRange.trim()

  if (cleaned.includes('||')) {
    const orParts = cleaned.split('||').map(part => part.trim())
    return orParts.some(part => isNodeVersionSufficient(part, minMajor))
  }

  const andParts = cleaned.split(AND_SPLIT_REGEX).filter(part => part !== '' && part !== '&&')
  if (andParts.length > 1) {
    for (const part of andParts) {
      const lowerBound = extractLowerBound(part)
      if (lowerBound !== null && lowerBound >= minMajor) return true
    }

    return andParts.every(part => couldIncludeVersion(part, minMajor))
  }

  return extractMajorAndCompare(cleaned, minMajor)
}

function extractLowerBound(range: string): number | null {
  const match = LOWER_BOUND_REGEX.exec(range)
  if (match) return Number.parseInt(match[1], 10)

  return null
}

function couldIncludeVersion(range: string, targetMajor: number): boolean {
  let match: RegExpMatchArray | null = null

  match = UPPER_BOUND_REGEX.exec(range)
  if (match) {
    const upperMajor = Number.parseInt(match[1], 10)
    return targetMajor <= upperMajor
  }

  return extractMajorAndCompare(range, targetMajor)
}

function extractMajorAndCompare(versionRange: string, minMajor: number): boolean {
  let match: RegExpMatchArray | null = null

  if (UPPER_BOUND_ONLY_REGEX.test(versionRange)) return false

  match = SEMVER_RANGE_REGEX.exec(versionRange)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = EXACT_SEMVER_REGEX.exec(versionRange)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = CARET_RANGE_REGEX.exec(versionRange)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = TILDE_RANGE_REGEX.exec(versionRange)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = MAJOR_MINOR_REGEX.exec(versionRange)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = WILDCARD_REGEX.exec(versionRange)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = MAJOR_ONLY_REGEX.exec(versionRange)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = NUMBER_ONLY_REGEX.exec(versionRange)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  return false
}

export const MIN_NODE_VERSION = '>=22.18.0'

export interface PackageJsonLike {
  version?: string
  engines?: Record<string, string>
  scripts?: Record<string, string>
  dependencies?: Record<string, string>
  [key: string]: unknown
}

function isPackageJsonLike(value: unknown): value is PackageJsonLike {
  return typeof value === 'object' && value !== null
}

function parsePackageJsonLike(raw: string): PackageJsonLike {
  const parsed: unknown = JSON.parse(raw)
  if (!isPackageJsonLike(parsed)) throw new Error('Invalid package.json')

  return parsed
}

/* oxlint-disable typescript/prefer-readonly-parameter-types -- intentionally mutates packageJson.engines in place */
export function ensureMinimumNodeEngine(
  packageJson: PackageJsonLike,
  minVersion: string = MIN_NODE_VERSION,
): boolean {
  packageJson.engines = packageJson.engines ?? {}

  if (packageJson.engines.node) {
    if (!satisfiesMinimumVersion(packageJson.engines.node, minVersion)) {
      logWarn(
        `Current engines.node value "${packageJson.engines.node}" may not meet the required minimum of ${minVersion}`,
      )
      logWarn(`Updating to ${minVersion} for deployment compatibility`)
      packageJson.engines.node = minVersion
      return true
    }
  } else {
    packageJson.engines.node = minVersion
    return true
  }

  return false
}
/* oxlint-enable typescript/prefer-readonly-parameter-types */

function satisfiesMinimumVersion(existingRange: string, requiredRange: string): boolean {
  const existingMin = extractMinimumVersion(existingRange)
  const requiredMin = extractMinimumVersion(requiredRange)

  if (!existingMin || !requiredMin) return false

  return compareVersions(existingMin, requiredMin) >= 0
}

function extractMinimumVersion(
  range: string,
): { major: number; minor: number; patch: number } | null {
  const cleaned = range.trim()

  const isNonNull = <T>(v: T | null): v is NonNullable<T> => v !== null

  if (cleaned.includes('||')) {
    const orParts = cleaned.split('||').map(part => part.trim())
    const versions = orParts.map(part => extractMinimumVersion(part)).filter(isNonNull)
    if (versions.length === 0) return null

    return versions.reduce((min, curr) => (compareVersions(curr, min) < 0 ? curr : min))
  }

  const andParts = cleaned.split(AND_SPLIT_REGEX).filter(part => part !== '' && part !== '&&')
  if (andParts.length > 1) {
    const lowerBounds = andParts
      .filter(part => LOWER_BOUND_REGEX.test(part))
      .map(part => extractMinimumVersion(part))
      .filter(isNonNull)

    if (lowerBounds.length === 0) return null

    return lowerBounds.reduce((max, curr) => (compareVersions(curr, max) > 0 ? curr : max))
  }

  let match: RegExpMatchArray | null = null

  match = SEMVER_RANGE_REGEX.exec(cleaned)
  if (match) {
    return {
      major: Number.parseInt(match[1], 10),
      minor: Number.parseInt(match[2], 10),
      patch: Number.parseInt(match[3], 10),
    }
  }

  match = EXACT_SEMVER_REGEX.exec(cleaned)
  if (match) {
    return {
      major: Number.parseInt(match[1], 10),
      minor: Number.parseInt(match[2], 10),
      patch: Number.parseInt(match[3], 10),
    }
  }

  match = CARET_RANGE_REGEX.exec(cleaned)
  if (match) {
    return {
      major: Number.parseInt(match[1], 10),
      minor: Number.parseInt(match[2], 10),
      patch: Number.parseInt(match[3], 10),
    }
  }

  match = TILDE_RANGE_REGEX.exec(cleaned)
  if (match) {
    return {
      major: Number.parseInt(match[1], 10),
      minor: Number.parseInt(match[2], 10),
      patch: Number.parseInt(match[3], 10),
    }
  }

  match = MAJOR_MINOR_REGEX.exec(cleaned)
  if (match) {
    return {
      major: Number.parseInt(match[1], 10),
      minor: Number.parseInt(match[2], 10),
      patch: 0,
    }
  }

  match = WILDCARD_REGEX.exec(cleaned)
  if (match) {
    return {
      major: Number.parseInt(match[1], 10),
      minor: 0,
      patch: 0,
    }
  }

  match = MAJOR_ONLY_REGEX.exec(cleaned)
  if (match) {
    return {
      major: Number.parseInt(match[1], 10),
      minor: 0,
      patch: 0,
    }
  }

  match = NUMBER_ONLY_REGEX.exec(cleaned)
  if (match) {
    return {
      major: Number.parseInt(match[1], 10),
      minor: 0,
      patch: 0,
    }
  }

  return null
}

function compareVersions(
  a: Readonly<{ major: number; minor: number; patch: number }>,
  b: Readonly<{ major: number; minor: number; patch: number }>,
): number {
  if (a.major !== b.major) return a.major - b.major
  if (a.minor !== b.minor) return a.minor - b.minor

  return a.patch - b.patch
}

export function getRariVersion(cwd: string = process.cwd()): string {
  const rariPackageJsonPath = join(cwd, 'node_modules/rari/package.json')

  if (!existsSync(rariPackageJsonPath)) {
    logError('rari is not installed. Please run "npm install rari" first.')
    process.exit(1)
    return ''
  }

  try {
    const packageJson = parsePackageJsonLike(readFileSync(rariPackageJsonPath, 'utf-8'))
    if (packageJson.version != null && packageJson.version !== '') {
      return `^${packageJson.version}`
    }

    logError('Could not determine rari version from package.json')
    process.exit(1)
    return ''
  } catch (error) {
    logError(
      `Failed to read rari package.json: ${error instanceof Error ? error.message : 'Unknown error'}`,
    )
    process.exit(1)
    return ''
  }
}

interface ProviderConfig {
  readonly providerName: string
  readonly deployScript: string
  readonly startScript?: string
  readonly dependency?: string
}

export type { ProviderConfig }

export function updatePackageJsonForProvider(cwd: string, config: ProviderConfig) {
  const packageJsonPath = join(cwd, 'package.json')
  if (!existsSync(packageJsonPath)) {
    logError('No package.json found. Please run this command from your project root.')
    process.exit(1)
  }

  try {
    const packageJson = parsePackageJsonLike(readFileSync(packageJsonPath, 'utf-8'))

    packageJson.scripts = packageJson.scripts ?? {}

    const newStart = config.startScript ?? 'rari start'
    if (
      packageJson.scripts.start &&
      packageJson.scripts.start !== newStart &&
      !packageJson.scripts['start:original']
    ) {
      logWarn(`Existing start script found: "${packageJson.scripts.start}"`)
      logWarn(`Backing up to start:original and replacing with "${newStart}"`)
      packageJson.scripts['start:original'] = packageJson.scripts.start
    }

    packageJson.scripts.start = newStart
    packageJson.scripts['start:local'] = 'rari start'
    packageJson.scripts[`deploy:${config.providerName.toLowerCase()}`] = config.deployScript

    ensureMinimumNodeEngine(packageJson)

    if (packageJson.dependencies?.rari == null || packageJson.dependencies.rari === '') {
      logInfo('Adding rari dependency...')
      packageJson.dependencies = packageJson.dependencies ?? {}
      packageJson.dependencies.rari = config.dependency ?? getRariVersion(cwd)
    }

    writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`)
    logSuccess(`Updated package.json for ${config.providerName} deployment`)
  } catch (error) {
    logError(
      `Failed to update package.json: ${error instanceof Error ? error.message : 'Unknown error'}`,
    )
    process.exit(1)
  }
}

export function updateGitignoreForProvider(
  cwd: string,
  providerName: string,
  providerFolder: string,
) {
  const gitignorePath = join(cwd, '.gitignore')
  const providerGitignoreEntries = ['', `# ${providerName}`, `${providerFolder}/`, ''].join('\n')

  if (existsSync(gitignorePath)) {
    const gitignoreContent = readFileSync(gitignorePath, 'utf-8')
    const lines = gitignoreContent.split('\n').map(line => line.trim())
    const hasExactMatch = lines.includes(`${providerFolder}/`) || lines.includes(providerFolder)

    if (!hasExactMatch) {
      writeFileSync(gitignorePath, gitignoreContent + providerGitignoreEntries)
      logSuccess(`Updated .gitignore with ${providerName} entries`)
    }
  } else {
    const defaultGitignore = `# Dependencies
node_modules
.pnpm-store

# Build outputs
dist

# Environment variables
.env
.env.local
.env.production

# ${providerName}
${providerFolder}

# Logs
*.log
npm-debug.log*
pnpm-debug.log*

# OS files
.DS_Store
Thumbs.db

# IDE files
.vscode
.idea
*.swp
*.swo
*~

# Temporary files
.tmp
tmp
`
    writeFileSync(gitignorePath, defaultGitignore)
    logSuccess(`Created .gitignore with ${providerName} entries`)
  }
}

export function createOrBackupConfigFile(cwd: string, filename: string, content: string) {
  const configPath = join(cwd, filename)

  try {
    if (existsSync(configPath)) {
      const existingConfig = readFileSync(configPath, 'utf-8')

      let backupFilename = `${filename}.backup`
      let backupPath = join(cwd, backupFilename)

      if (existsSync(backupPath)) {
        const timestamp = Date.now()
        backupFilename = `${filename}.backup.${timestamp}`
        backupPath = join(cwd, backupFilename)
      }

      logWarn(`${filename} already exists, backing up to ${backupFilename}`)
      writeFileSync(backupPath, existingConfig, { flag: 'wx' })
    }

    writeFileSync(configPath, content)
    logSuccess(`Created ${filename} configuration`)
  } catch (error) {
    logError(
      `Failed to create or backup ${filename}: ${error instanceof Error ? error.message : 'Unknown error'}`,
    )
    process.exit(1)
  }
}
