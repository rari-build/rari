import process from 'node:process'
import { styleText } from 'node:util'
import {
  createOrBackupConfigFile,
  logInfo,
  printDeploymentSuccess,
  updatePackageJsonForProvider,
} from './utils'

export function createRenderDeployment() {
  const cwd = process.cwd()

  logInfo('Creating Render deployment configuration...')

  updatePackageJsonForProvider(cwd, { providerName: 'Render' })

  createRenderYaml(cwd)

  printDeploymentSuccess(
    'Render',
    [
      styleText('cyan', 'git add .'),
      styleText('cyan', 'git commit -m "Add Render deployment"'),
      styleText('cyan', 'git push origin main'),
      `Go to ${styleText('cyan', 'https://render.com')} and create a Web Service`,
    ],
    ['Bind to 0.0.0.0 (Render requirement)', "Use Render's PORT environment variable"],
  )
}

function createRenderYaml(cwd: string) {
  const renderConfig = `services:
  - type: web
    name: rari-app
    runtime: node
    plan: free
    buildCommand: npm install --include=dev && npx rari build
    startCommand: npm start
    healthCheckPath: /_rari/health
    envVars:
      - key: NODE_ENV
        value: production
      - key: RUST_LOG
        value: info
`

  createOrBackupConfigFile(cwd, 'render.yaml', renderConfig)
}
