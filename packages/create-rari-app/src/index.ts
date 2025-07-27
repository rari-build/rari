#!/usr/bin/env node
// oxlint-disable no-console
import { spawn } from 'node:child_process'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import process from 'node:process'
import { cancel, confirm, intro, isCancel, outro, select, spinner, text } from '@clack/prompts'
import pc from 'picocolors'

interface ProjectOptions {
  name: string
  template: string
  packageManager: string
  installDeps: boolean
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

async function main() {
  console.clear()

  intro(pc.bgCyan(pc.black(' create-rari-app ')))

  const projectName = await text({
    message: 'What is your project named?',
    placeholder: 'my-rari-app',
    validate: (value) => {
      if (!value)
        return 'Please enter a project name.'
      if (value.includes(' '))
        return 'Project name cannot contain spaces.'
      if (!/^[\w-]+$/.test(value))
        return 'Project name can only contain letters, numbers, hyphens, and underscores.'
    },
  })

  if (isCancel(projectName)) {
    cancel('Operation cancelled.')
    process.exit(0)
  }

  const template = await select({
    message: 'Which template would you like to use?',
    options: Object.entries(templates).map(([key, { name, description }]) => ({
      value: key,
      label: name,
      hint: description,
    })),
  })

  if (isCancel(template)) {
    cancel('Operation cancelled.')
    process.exit(0)
  }

  const packageManager = await select({
    message: 'Which package manager would you like to use?',
    options: Object.entries(packageManagers).map(([key, value]) => ({
      value: key,
      label: value,
    })),
  })

  if (isCancel(packageManager)) {
    cancel('Operation cancelled.')
    process.exit(0)
  }

  const installDeps = await confirm({
    message: 'Install dependencies?',
    initialValue: true,
  })

  if (isCancel(installDeps)) {
    cancel('Operation cancelled.')
    process.exit(0)
  }

  const options: ProjectOptions = {
    name: projectName,
    template: template as string,
    packageManager: packageManager as string,
    installDeps,
  }

  await createProject(options)

  outro(pc.green('🎉 Project created successfully!'))

  console.log()
  console.log(pc.cyan('Next steps:'))
  console.log(pc.gray(`  cd ${options.name}`))

  if (!options.installDeps) {
    console.log(pc.gray(`  ${options.packageManager} install`))
  }

  console.log(pc.gray(`  ${options.packageManager} run dev`))
  console.log()
}

async function createProject(options: ProjectOptions) {
  const projectPath = join(process.cwd(), options.name)
  const templatePath = join(import.meta.dirname, '..', 'templates', options.template)

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
  }
  catch (error) {
    s.stop('Error occurred.')
    throw error
  }
}

async function copyTemplate(templatePath: string, projectPath: string, options: ProjectOptions) {
  const templateFiles = [
    'package.json',
    'vite.config.ts',
    'tsconfig.json',
    'tsconfig.app.json',
    'tsconfig.node.json',
    'index.html',
    'src/main.tsx',
    'src/App.tsx',
    'src/vite-env.d.ts',
    'src/styles/index.css',
    'src/components/Welcome.tsx',
    'src/components/ServerTime.tsx',
    '.gitignore',
  ]

  // Create directories
  await mkdir(join(projectPath, 'src', 'components'), { recursive: true })
  await mkdir(join(projectPath, 'src', 'styles'), { recursive: true })

  for (const file of templateFiles) {
    const sourcePath = join(templatePath, file)
    const destPath = join(projectPath, file)

    try {
      let content = await readFile(sourcePath, 'utf-8')

      // Replace template variables
      content = content
        .replace(/\{\{PROJECT_NAME\}\}/g, options.name)
        .replace(/\{\{PACKAGE_MANAGER\}\}/g, options.packageManager)

      await mkdir(dirname(destPath), { recursive: true })
      await writeFile(destPath, content)
    }
    catch (error) {
      console.warn(`Warning: Could not copy ${file}:`, error)
    }
  }
}

async function installDependencies(projectPath: string, packageManager: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const child = spawn(packageManager, ['install'], {
      cwd: projectPath,
      stdio: 'pipe',
    })

    child.on('close', (code) => {
      if (code === 0) {
        resolve()
      }
      else {
        reject(new Error(`${packageManager} install failed with code ${code}`))
      }
    })

    child.on('error', reject)
  })
}

main().catch((error) => {
  console.error(pc.red('Error:'), error.message)
  process.exit(1)
})
