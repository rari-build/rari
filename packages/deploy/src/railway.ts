import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import { styleText } from 'node:util'
import { isNodeVersionSufficient, logError, logInfo, logSuccess, logWarn } from './utils'

export async function createRailwayDeployment() {
  const cwd = process.cwd()

  logInfo('Creating Railway deployment configuration...')

  const packageJsonPath = join(cwd, 'package.json')
  if (!existsSync(packageJsonPath)) {
    logError('No package.json found. Please run this command from your project root.')
    process.exit(1)
  }

  try {
    const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'))

    packageJson.scripts = packageJson.scripts || {}

    if (packageJson.scripts.start && packageJson.scripts.start !== 'rari start') {
      logWarn(`Existing start script found: "${packageJson.scripts.start}"`)
      logWarn('Backing up to start:original and replacing with "rari start"')
      packageJson.scripts['start:original'] = packageJson.scripts.start
    }

    packageJson.scripts.start = 'rari start'
    packageJson.scripts['start:local'] = 'rari start'
    packageJson.scripts['deploy:railway'] = 'echo "Push to GitHub and connect to Railway to deploy"'

    packageJson.engines = packageJson.engines || {}
    if (packageJson.engines.node) {
      if (!isNodeVersionSufficient(packageJson.engines.node)) {
        logWarn(`Current engines.node value "${packageJson.engines.node}" may not meet the required minimum of >=22.0.0`)
        logWarn('Updating to >=22.0.0 for Railway deployment compatibility')
        packageJson.engines.node = '>=22.0.0'
      }
    }
    else {
      packageJson.engines.node = '>=22.0.0'
    }

    if (!packageJson.dependencies || !packageJson.dependencies.rari) {
      logInfo('Adding rari dependency...')
      packageJson.dependencies = packageJson.dependencies || {}
      packageJson.dependencies.rari = '^0.1.0'
    }

    writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`)
    logSuccess('Updated package.json for Railway deployment')
  }
  catch (error) {
    logError(`Failed to update package.json: ${error instanceof Error ? error.message : 'Unknown error'}`)
    process.exit(1)
  }

  const railwayConfig = `[build]
builder = "RAILPACK"

[deploy]
startCommand = "npm start"
healthcheckPath = "/"
healthcheckTimeout = 300
restartPolicyType = "ON_FAILURE"
restartPolicyMaxRetries = 3
`

  const railwayTomlPath = join(cwd, 'railway.toml')
  if (existsSync(railwayTomlPath)) {
    logWarn('railway.toml already exists, backing up to railway.toml.backup')
    const existingConfig = readFileSync(railwayTomlPath, 'utf-8')
    writeFileSync(join(cwd, 'railway.toml.backup'), existingConfig)
  }

  writeFileSync(railwayTomlPath, railwayConfig)
  logSuccess('Created railway.toml configuration')

  const gitignorePath = join(cwd, '.gitignore')
  const railwayGitignoreEntries = [
    '',
    '# Railway',
    '.railway/',
    '',
  ].join('\n')

  if (existsSync(gitignorePath)) {
    const gitignoreContent = readFileSync(gitignorePath, 'utf-8')
    if (!gitignoreContent.includes('.railway/')) {
      writeFileSync(gitignorePath, gitignoreContent + railwayGitignoreEntries)
      logSuccess('Updated .gitignore with Railway entries')
    }
  }
  else {
    const defaultGitignore = `# Dependencies
node_modules/
.pnpm-store/

# Build outputs
dist/

# Environment variables
.env
.env.local
.env.production

# Railway
.railway/

# Logs
*.log
npm-debug.log*
pnpm-debug.log*

# OS files
.DS_Store
Thumbs.db

# IDE files
.vscode/
.idea/
*.swp
*.swo
*~

# Temporary files
.tmp/
tmp/
`
    writeFileSync(gitignorePath, defaultGitignore)
    logSuccess('Created .gitignore with Railway entries')
  }

  const readmePath = join(cwd, 'README.md')
  const railwayReadmeSection = `
## 🚂 Deploy to Railway

This rari application is configured for Railway deployment.

### Quick Deploy

1. **Push to GitHub**:
   \`\`\`bash
   git add .
   git commit -m "Add Railway deployment"
   git push origin main
   \`\`\`

2. **Deploy to Railway**:
   - Go to [railway.app](https://railway.app)
   - Create new project → "Deploy from GitHub repo"
   - Select your repository
   - Click "Deploy Now"

3. **Generate Domain**:
   - In Railway dashboard → Settings → Networking
   - Click "Generate Domain"
   - Your app will be live! 🎉

### Local Development

\`\`\`bash
# Development server
npm run start:local

# Production simulation
npm start
\`\`\`

### Environment Variables

Railway automatically provides:
- \`PORT\` - Server port (Railway assigns this)
- \`NODE_ENV=production\` - Production mode

Optional variables you can set:
- \`RUST_LOG=debug\` - Rust logging level

---
`

  if (existsSync(readmePath)) {
    const readmeContent = readFileSync(readmePath, 'utf-8')
    if (!readmeContent.includes('Deploy to Railway')) {
      writeFileSync(readmePath, readmeContent + railwayReadmeSection)
      logSuccess('Updated README.md with Railway deployment instructions')
    }
  }
  else {
    const defaultReadme = `# My rari App

A high-performance React Server Components application powered by rari.
${railwayReadmeSection}
## Getting Started

\`\`\`bash
npm install
npm start
\`\`\`

Visit [http://localhost:3000](http://localhost:3000) to see your app.
`
    writeFileSync(readmePath, defaultReadme)
    logSuccess('Created README.md with Railway deployment instructions')
  }

  console.warn('')
  logSuccess('Railway deployment setup complete! 🎉')
  console.warn('')
  logInfo('Next steps:')
  console.warn(`  1. ${styleText('cyan', 'git add .')}`)
  console.warn(`  2. ${styleText('cyan', 'git commit -m "Add Railway deployment"')}`)
  console.warn(`  3. ${styleText('cyan', 'git push origin main')}`)
  console.warn(`  4. Go to ${styleText('cyan', 'https://railway.app')} and deploy from GitHub`)
  console.warn('')
  logInfo('Your rari app will automatically:')
  console.warn('  ✅ Detect Railway environment')
  console.warn('  ✅ Bind to 0.0.0.0 (Railway requirement)')
  console.warn('  ✅ Use Railway\'s PORT environment variable')
  console.warn('  ✅ Run in production mode')
  console.warn('  ✅ Download platform-specific rari binary')
  console.warn('')
  logSuccess('Ready for deployment! 🚀')
}
