import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import { styleText } from 'node:util'
import { createOrBackupConfigFile, getRariVersion, logInfo, logSuccess, updateGitignoreForProvider, updatePackageJsonForProvider } from './utils'

export function createRailwayDeployment() {
  const cwd = process.cwd()

  logInfo('Creating Railway deployment configuration...')

  updatePackageJsonForProvider(cwd, {
    providerName: 'Railway',
    deployScript: 'echo "Push to GitHub and connect to Railway to deploy"',
    startScript: 'rari start',
    dependency: getRariVersion(),
  })

  createRailwayToml(cwd)

  updateGitignoreForProvider(cwd, 'Railway', '.railway')

  updateReadmeForRailway(cwd)

  printRailwaySuccessMessage()
}

function createRailwayToml(cwd: string) {
  const railwayConfig = `[build]
builder = "RAILPACK"

[deploy]
startCommand = "npm start"
healthcheckPath = "/_rari/health"
healthcheckTimeout = 300
restartPolicyType = "ON_FAILURE"
restartPolicyMaxRetries = 3
`

  createOrBackupConfigFile(cwd, 'railway.toml', railwayConfig)
}

function updateReadmeForRailway(cwd: string) {
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
}

function printRailwaySuccessMessage() {
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
