import { existsSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import { styleText } from 'node:util'
import {
  createOrBackupConfigFile,
  getInstallCommand,
  logInfo,
  logSuccess,
  printDeploymentSuccess,
  updatePackageJsonForProvider,
} from './utils'

export function createFlyDeployment() {
  const cwd = process.cwd()

  logInfo('Creating Fly.io deployment configuration...')

  const addedRari = updatePackageJsonForProvider(cwd, {
    providerName: 'Fly',
    deployScript: 'fly deploy',
  })

  createFlyToml(cwd)
  createDockerfile(cwd)
  createDockerignore(cwd)

  printDeploymentSuccess(
    'Fly.io',
    [
      ...(addedRari
        ? [`${styleText('cyan', getInstallCommand(cwd))} (updates your lockfile for rari)`]
        : []),
      `Install the Fly CLI: ${styleText('cyan', 'https://fly.io/docs/flyctl/install/')}`,
      styleText('cyan', 'fly auth login'),
      styleText('cyan', 'fly launch --no-deploy'),
      styleText('cyan', 'fly deploy'),
    ],
    [
      'Run in a Node 24 Docker container',
      'Bind to 0.0.0.0 on port 8080',
      'Health check on /_rari/health',
    ],
  )
}

function createFlyToml(cwd: string) {
  const flyConfig = `app = "rari-app"
primary_region = "iad"

[build]

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = "stop"
  auto_start_machines = true
  min_machines_running = 0

  [[http_service.checks]]
    grace_period = "30s"
    interval = "15s"
    method = "GET"
    path = "/_rari/health"
    timeout = "5s"

[[vm]]
  memory = "512mb"
  cpu_kind = "shared"
  cpus = 1
`

  createOrBackupConfigFile(cwd, 'fly.toml', flyConfig)
}

interface DockerInstallSteps {
  readonly copy: string
  readonly install: readonly string[]
}

function getDockerInstallSteps(cwd: string): DockerInstallSteps {
  const has = (file: string) => existsSync(join(cwd, file))

  if (has('pnpm-lock.yaml')) {
    return {
      copy: 'package.json pnpm-lock.yaml',
      install: [
        'ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0',
        'RUN corepack enable',
        'RUN pnpm install --frozen-lockfile',
      ],
    }
  }
  if (has('yarn.lock')) {
    return {
      copy: 'package.json yarn.lock',
      install: [
        'ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0',
        'ENV YARN_NODE_LINKER=node-modules',
        'RUN corepack enable',
        'RUN yarn install',
      ],
    }
  }
  if (has('bun.lock') || has('bun.lockb')) {
    return {
      copy: `package.json ${has('bun.lock') ? 'bun.lock' : 'bun.lockb'}`,
      install: ['RUN npm install -g bun', 'RUN bun install --frozen-lockfile'],
    }
  }
  if (has('package-lock.json')) {
    return { copy: 'package.json package-lock.json', install: ['RUN npm ci'] }
  }

  return { copy: 'package.json', install: ['RUN npm install'] }
}

function createDockerfile(cwd: string) {
  const { copy, install } = getDockerInstallSteps(cwd)
  const dockerfile = `FROM node:24-slim

RUN apt-get update && apt-get install -y libfontconfig1 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY ${copy} ./
${install.join('\n')}

COPY . .
RUN npx rari build

ENV NODE_ENV=production
ENV PORT=8080
ENV RARI_HOST=0.0.0.0
ENV RUST_LOG=info

EXPOSE 8080

CMD ["npm", "start"]
`

  createOrBackupConfigFile(cwd, 'Dockerfile', dockerfile)
}

function createDockerignore(cwd: string) {
  const dockerignorePath = join(cwd, '.dockerignore')
  if (existsSync(dockerignorePath)) return

  writeFileSync(
    dockerignorePath,
    `node_modules
.git
.env
.env.*
!.env.example
dist
*.log
.DS_Store
.vscode
.idea
`,
  )
  logSuccess('Created .dockerignore configuration')
}
