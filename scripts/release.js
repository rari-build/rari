import fs from 'node:fs/promises'
import path from 'node:path'
import process from 'node:process'
import { cancel, intro, isCancel, outro, select, text } from '@clack/prompts'
import colors from 'picocolors'
import semver from 'semver'
import { logRecentCommits, run } from './releaseUtils.js'

const args = process.argv.slice(2)
const skipPrompts = args.includes('--skip-prompts')

const packages = [
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
  {
    name: 'rari-linux-x64',
    path: 'packages/rari-linux-x64',
    needsBuild: false,
  },
  {
    name: 'rari-linux-arm64',
    path: 'packages/rari-linux-arm64',
    needsBuild: false,
  },
  {
    name: 'rari-darwin-x64',
    path: 'packages/rari-darwin-x64',
    needsBuild: false,
  },
  {
    name: 'rari-darwin-arm64',
    path: 'packages/rari-darwin-arm64',
    needsBuild: false,
  },
  {
    name: 'rari-win32-x64',
    path: 'packages/rari-win32-x64',
    needsBuild: false,
  },
]

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
    '--output',
    'CHANGELOG.md',
  ]
  await run('npx', changelogArgs, { cwd: pkgPath })

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
