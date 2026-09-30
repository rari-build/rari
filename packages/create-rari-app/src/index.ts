import { spawn } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import { styleText } from 'node:util'
import { cancel, confirm, intro, isCancel, outro, select, spinner, text } from '@clack/prompts'

const TEMPLATE_PLACEHOLDER_REGEX = /\{\{PROJECT_NAME\}\}/g
const PACKAGE_MANAGER_PLACEHOLDER_REGEX = /\{\{PACKAGE_MANAGER\}\}/g
const INSTALL_COMMAND_PLACEHOLDER_REGEX = /\{\{INSTALL_COMMAND\}\}/g
const PACKAGE_MANAGER_SPEC_PLACEHOLDER_REGEX = /\{\{PACKAGE_MANAGER_SPEC\}\}/g
const PACKAGE_MANAGER_FIELD_REGEX = /\n\s*"packageManager": "[^"]*",/
const PROJECT_NAME_REGEX = /^[@\w/-]+$/
const PACKAGE_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')

function readPnpmPackageManager(): string | null {
  const parsed: unknown = JSON.parse(readFileSync(join(PACKAGE_ROOT, 'package.json'), 'utf8'))
  if (typeof parsed !== 'object' || parsed == null || !('packageManager' in parsed)) return null
  return typeof parsed.packageManager === 'string' ? parsed.packageManager : null
}

const PNPM_PACKAGE_MANAGER = readPnpmPackageManager()

interface ProjectOptions {
  readonly name: string
  readonly template: string
  readonly packageManager: string
  readonly installDeps: boolean
}

const templates = {
  default: {
    name: 'Default',
    description: 'A clean starter with React Server Components',
  },
} as const

const packageManagers = {
  pnpm: 'pnpm',
  npm: 'npm',
  yarn: 'yarn',
  bun: 'bun',
} as const

function installCommandFor(packageManager: string): string {
  switch (packageManager) {
    case 'yarn':
      return 'yarn'
    case 'bun':
      return 'bun install'
    case 'npm':
      return 'npm install'
    default:
      return 'pnpm install'
  }
}

function packageManagerSpecFor(packageManager: string): string | null {
  return packageManager === 'pnpm' && PNPM_PACKAGE_MANAGER != null ? PNPM_PACKAGE_MANAGER : null
}

function requireAnswer<T>(value: T): Exclude<T, symbol> {
  if (isCancel(value)) {
    cancel('Operation cancelled.')
    process.exit(0)
  }
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion isCancel narrows cancel symbol; process.exit never returns
  return value as Exclude<T, symbol>
}

async function main() {
  intro(styleText(['bgCyan', 'black'], ' create-rari-app '))

  const args = process.argv.slice(2)
  let projectName = args[0]

  if (projectName) {
    if (projectName.includes(' ')) {
      console.error(styleText('red', 'Error: Project name cannot contain spaces.'))
      process.exit(1)
    }
    if (!PROJECT_NAME_REGEX.test(projectName)) {
      console.error(
        styleText(
          'red',
          'Error: Project name can only contain letters, numbers, hyphens, underscores, slashes, and @ symbol.',
        ),
      )
      process.exit(1)
    }
  } else {
    projectName = requireAnswer(
      await text({
        message: 'What is your project named?',
        placeholder: 'my-rari-app',
        validate: value => {
          if (value == null || value === '') return 'Please enter a project name.'
          if (value.includes(' ')) return 'Project name cannot contain spaces.'
          if (!PROJECT_NAME_REGEX.test(value))
            return 'Project name can only contain letters, numbers, hyphens, underscores, slashes, and @ symbol.'

          return undefined
        },
      }),
    )
  }

  const template = requireAnswer(
    await select({
      message: 'Which template would you like to use?',
      options: Object.entries(templates).map(([key, { name, description }]) => ({
        value: key,
        label: name,
        hint: description,
      })),
    }),
  )

  const packageManager = requireAnswer(
    await select({
      message: 'Which package manager would you like to use?',
      options: Object.entries(packageManagers).map(([key, value]) => ({
        value: key,
        label: value,
      })),
    }),
  )

  const installDeps = requireAnswer(
    await confirm({
      message: 'Install dependencies?',
      initialValue: true,
    }),
  )

  const options: ProjectOptions = {
    name: projectName,
    template,
    packageManager,
    installDeps,
  }

  await createProject(options)

  outro(styleText('green', '🎉 Project created successfully!'))

  console.warn()
  console.warn(styleText('cyan', 'Next steps:'))
  console.warn(styleText('gray', `  cd ${options.name}`))

  if (!options.installDeps) console.warn(styleText('gray', `  ${options.packageManager} install`))

  console.warn(styleText('gray', `  ${options.packageManager} run dev`))
  console.warn()
}

async function createProject(options: ProjectOptions) {
  const projectPath = join(process.cwd(), options.name)
  const templatePath = join(
    dirname(fileURLToPath(import.meta.url)),
    '..',
    'templates',
    options.template,
  )

  const s = spinner()

  try {
    s.start('Creating project structure...')
    await mkdir(projectPath, { recursive: true })
    await copyTemplate(templatePath, projectPath, options)
    s.stop('Project structure created.')

    if (options.installDeps) {
      s.start('Installing dependencies...')
      await installDependencies(projectPath, options.packageManager)
      s.stop('Dependencies installed.')
    }
  } catch (error) {
    s.stop('Error occurred.')
    throw error
  }
}

async function copyTemplate(templatePath: string, projectPath: string, options: ProjectOptions) {
  const templateFiles = [
    'package.json',
    'vite.config.ts',
    'tsconfig.json',
    'README.md',
    'src/app/globals.css',
    'src/app/layout.tsx',
    'src/app/page.tsx',
    'src/app/robots.ts',
    'src/app/about/page.tsx',
    'src/components/Welcome.tsx',
    'src/components/ServerTime.tsx',
    'src/components/Rari.tsx',
    'src/components/SiteNav.tsx',
    'gitignore',
  ]

  await mkdir(join(projectPath, 'src', 'app', 'about'), { recursive: true })
  await mkdir(join(projectPath, 'src', 'components'), { recursive: true })

  const installCommand = installCommandFor(options.packageManager)
  const packageManagerSpec = packageManagerSpecFor(options.packageManager)

  for (const file of templateFiles) {
    const sourcePath = join(templatePath, file)
    const destFile = file === 'gitignore' ? '.gitignore' : file
    const destPath = join(projectPath, destFile)

    try {
      let content = await readFile(sourcePath, 'utf-8')

      content = content
        .replace(TEMPLATE_PLACEHOLDER_REGEX, options.name)
        .replace(PACKAGE_MANAGER_PLACEHOLDER_REGEX, options.packageManager)
        .replace(INSTALL_COMMAND_PLACEHOLDER_REGEX, installCommand)

      if (file === 'package.json') {
        content =
          packageManagerSpec == null
            ? content.replace(PACKAGE_MANAGER_FIELD_REGEX, '')
            : content.replace(PACKAGE_MANAGER_SPEC_PLACEHOLDER_REGEX, packageManagerSpec)
      }

      await mkdir(dirname(destPath), { recursive: true })
      await writeFile(destPath, content)
    } catch (error) {
      console.warn(`Warning: Could not copy ${file}:`, error)
    }
  }
}

async function installDependencies(projectPath: string, packageManager: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const child = spawn(packageManager, ['install'], {
      cwd: projectPath,
      stdio: 'pipe',
      shell: process.platform === 'win32',
    })

    child.on('close', (code: number | null) => {
      if (code === 0) resolve()
      else reject(new Error(`${packageManager} install failed with code ${code}`))
    })

    child.on('error', reject)
  })
}

main().catch((error: unknown) => {
  console.error(styleText('red', 'Error:'), error instanceof Error ? error.message : String(error))
  process.exit(1)
})
