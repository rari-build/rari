import fs from 'node:fs/promises'
import { execa } from 'execa'
import colors from 'picocolors'

export function run(bin, args, opts = {}) {
  return execa(bin, args, { stdio: 'inherit', ...opts })
}

export async function getLatestTag(pkgName) {
  const pkgDir = `packages/${pkgName}`
  const pkgJson = JSON.parse(
    await fs.readFile(`${pkgDir}/package.json`, 'utf-8'),
  )
  const version = pkgJson.version
  return `${pkgName}@${version}`
}

export async function logRecentCommits(pkgName) {
  const tag = await getLatestTag(pkgName)
  if (!tag)
    return

  try {
    const sha = await run('git', ['rev-list', '-n', '1', tag], {
      stdio: 'pipe',
    }).then(res => res.stdout.trim())

    console.warn(
      colors.bold(
        `\n${colors.blue(`i`)} Commits of ${colors.green(
          pkgName,
        )} since ${colors.green(tag)} ${colors.gray(`(${sha.slice(0, 5)})`)}`,
      ),
    )

    const pkgDir = `packages/${pkgName}`
    await run(
      'git',
      [
        '--no-pager',
        'log',
        `${sha}..HEAD`,
        '--oneline',
        '--',
        pkgDir,
      ],
      { stdio: 'inherit' },
    )
    console.warn()
  }
  catch {
    console.warn(colors.yellow(`No previous tag found for ${pkgName}, showing all commits...`))
    const pkgDir = `packages/${pkgName}`
    await run(
      'git',
      [
        '--no-pager',
        'log',
        '--oneline',
        '--',
        pkgDir,
      ],
      { stdio: 'inherit' },
    )
  }
}


