import fs from 'node:fs/promises'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import { cancel, intro, isCancel, outro, select, text } from '@clack/prompts'
import colors from 'picocolors'
import semver from 'semver'
import { logRecentCommits, run } from './releaseUtils.js'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const projectRoot = path.resolve(__dirname, '..')

const args = process.argv.slice(2)
const skipPrompts = args.includes('--skip-prompts')

let packages = [
  {
    name: 'rari',
    path: 'packages/rari',
    needsBuild: true,
  },
  {
    name: 'create-rari-app',
    path: 'packages/create-rari-app',
    needsBuild: true,
  },
]

// Allow selecting a subset of packages via CLI or env
// Examples:
//   node scripts/release.js --only rari
//   node scripts/release.js --only rari,create-rari-app
const onlyArgIdx = args.findIndex(a => a === '--only' || a.startsWith('--only='))
let onlyList = null
if (onlyArgIdx !== -1) {
  const val = args[onlyArgIdx].includes('=') ? args[onlyArgIdx].split('=')[1] : args[onlyArgIdx + 1]
  if (val) {
    onlyList = val.split(',').map(s => s.trim()).filter(Boolean)
  }
}
if (!onlyList && process.env.PACKAGES) {
  onlyList = process.env.PACKAGES.split(',').map(s => s.trim()).filter(Boolean)
}
if (onlyList && onlyList.length > 0) {
  packages = packages.filter(p => onlyList.includes(p.name))
  if (packages.length === 0) {
    console.error(colors.red(`No matching packages for selection: ${onlyList.join(', ')}`))
    process.exit(1)
  }
}

async function release() {
  intro(colors.cyan('🚀 Rari Release Script'))

  for (const pkg of packages) {
    await releasePackage(pkg)
  }

  outro(colors.green('✨ All packages released successfully!'))
}

async function releasePackage(pkg) {
  const pkgPath = path.resolve(pkg.path)
  const pkgJsonPath = path.join(pkgPath, 'package.json')

  console.warn(colors.bold(`\n📦 Releasing ${colors.cyan(pkg.name)}`))

  const pkgJson = JSON.parse(await fs.readFile(pkgJsonPath, 'utf-8'))
  const currentVersion = pkgJson.version

  await logRecentCommits(pkg.name)

  const newVersion = await getNewVersion(currentVersion, skipPrompts)

  if (newVersion === currentVersion) {
    console.warn(colors.yellow('No version change, skipping release'))
    return
  }

  console.warn(colors.cyan(`\nReleasing ${pkg.name} v${newVersion}...`))

  if (pkg.needsBuild) {
    console.warn(colors.cyan('\nBuilding package...'))
    await run('pnpm', ['build'], { cwd: pkgPath })
  }
  else {
    console.warn(colors.cyan('\nSkipping build for platform package...'))
  }

  pkgJson.version = newVersion
  await fs.writeFile(pkgJsonPath, `${JSON.stringify(pkgJson, null, 2)}\n`)

  console.warn(colors.cyan('\nGenerating changelog...'))

  const changelogArgs = [
    'git-cliff',
    '--tag',
    `v${newVersion}`,
    '--output',
    'CHANGELOG.md',
  ]
  await run('pnpm', ['exec', ...changelogArgs], { cwd: projectRoot })

  // Copy the generated changelog to the package directory
  const sourceChangelogPath = path.join(projectRoot, 'CHANGELOG.md')
  const targetChangelogPath = path.join(pkgPath, 'CHANGELOG.md')
  await fs.copyFile(sourceChangelogPath, targetChangelogPath)

  await run('git', ['add', '.'], { cwd: pkgPath })
  await run('git', ['commit', '-m', `release: ${pkg.name}@${newVersion}`])

  const tag = `${pkg.name}@${newVersion}`
  await run('git', ['tag', tag])

  console.warn(colors.cyan('\nPublishing to npm...'))
  const publishArgs = ['publish', '--access', 'public']

  const isPrerelease = semver.prerelease(newVersion)
  if (isPrerelease) {
    publishArgs.push('--tag', 'next')
  }

  await run('npm', publishArgs, { cwd: pkgPath })

  console.warn(colors.green(`\n✅ Released ${pkg.name}@${newVersion}`))
}

async function getNewVersion(currentVersion, skipPrompts) {
  // Allow non-interactive control via env vars
  const envVersion = process.env.RELEASE_VERSION
  const envType = process.env.RELEASE_TYPE
  if (envVersion) {
    if (!semver.valid(envVersion)) {
      throw new Error(`Invalid RELEASE_VERSION: ${envVersion}`)
    }
    if (!semver.gt(envVersion, currentVersion)) {
      throw new Error(`RELEASE_VERSION (${envVersion}) must be greater than current version ${currentVersion}`)
    }
    return envVersion
  }
  if (envType) {
    const allowedTypes = new Set([
      'patch',
      'minor',
      'major',
      'prepatch',
      'preminor',
      'premajor',
      'prerelease',
    ])
    if (!allowedTypes.has(envType)) {
      throw new Error(`Invalid RELEASE_TYPE: ${envType}`)
    }
    return semver.inc(currentVersion, envType)
  }
  if (skipPrompts) {
    return currentVersion
  }

  const versionIncrements = [
    { value: 'patch', label: `patch (${semver.inc(currentVersion, 'patch')})` },
    { value: 'minor', label: `minor (${semver.inc(currentVersion, 'minor')})` },
    { value: 'major', label: `major (${semver.inc(currentVersion, 'major')})` },
    {
      value: 'prepatch',
      label: `prepatch (${semver.inc(currentVersion, 'prepatch')})`,
    },
    {
      value: 'preminor',
      label: `preminor (${semver.inc(currentVersion, 'preminor')})`,
    },
    {
      value: 'premajor',
      label: `premajor (${semver.inc(currentVersion, 'premajor')})`,
    },
    {
      value: 'prerelease',
      label: `prerelease (${semver.inc(currentVersion, 'prerelease')})`,
    },
    { value: 'custom', label: 'custom' },
  ]

  const releaseType = await select({
    message: `Select release type (current: ${currentVersion}):`,
    options: versionIncrements,
  })

  if (isCancel(releaseType)) {
    cancel('Release cancelled')
    process.exit(0)
  }

  if (releaseType === 'custom') {
    const customVersion = await text({
      message: 'Enter custom version:',
      validate: (input) => {
        if (!semver.valid(input)) {
          return 'Please enter a valid semantic version'
        }
        if (!semver.gt(input, currentVersion)) {
          return `Version must be greater than current version ${currentVersion}`
        }
      },
    })

    if (isCancel(customVersion)) {
      cancel('Release cancelled')
      process.exit(0)
    }

    return customVersion
  }

  return semver.inc(currentVersion, releaseType)
}

process.on('unhandledRejection', (error) => {
  console.error(colors.red('\nUnhandled error:'), error)
  process.exit(1)
})

release().catch((error) => {
  console.error(colors.red('\nRelease failed:'), error)
  process.exit(1)
})
