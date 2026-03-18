import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import { styleText } from 'node:util'
import { createOrBackupConfigFile, getRariVersion, logInfo, logSuccess, updateGitignoreForProvider, updatePackageJsonForProvider } from './utils'

export function createRenderDeployment() {
  const cwd = process.cwd()

  logInfo('Creating Render deployment configuration...')

  updatePackageJsonForProvider(cwd, {
    providerName: 'Render',
    deployScript: 'echo "Push to GitHub and connect to Render to deploy"',
    startScript: 'rari start',
    dependency: getRariVersion(),
  })

  createRenderYaml(cwd)

  updateGitignoreForProvider(cwd, 'Render', '.render')

  updateReadmeForRender(cwd)

  printRenderSuccessMessage()
}

function createRenderYaml(cwd: string) {
  const renderConfig = `services:
  - type: web
    name: rari-app
    runtime: node
    env: node
    plan: free
    buildCommand: npm install
    startCommand: npm start
    healthCheckPath: /
    envVars:
      - key: NODE_ENV
        value: production
      - key: RUST_LOG
        value: info
`

  createOrBackupConfigFile(cwd, 'render.yaml', renderConfig)
}

function updateReadmeForRender(cwd: string) {
  const readmePath = join(cwd, 'README.md')
  const renderReadmeSection = `
## 🎨 Deploy to Render

This rari application is configured for Render deployment.

### Quick Deploy

1. **Push to GitHub**:
   \`\`\`bash
   git add .
   git commit -m "Add Render deployment"
   git push origin main
   \`\`\`

2. **Deploy to Render**:
   - Go to [render.com](https://render.com)
   - Create new "Web Service"
   - Connect your GitHub repository
   - Render will auto-detect Node.js and use the configuration from \`render.yaml\`
   - Click "Create Web Service"

3. **Your app will be live!** 🎉
   - Render provides a \`.onrender.com\` URL
   - Optional: Add custom domain in Render dashboard

### Local Development

\`\`\`bash
# Development server
npm run start:local

# Production simulation
npm start
\`\`\`

### Environment Variables

Render automatically provides:
- \`PORT\` - Server port (Render assigns this)
- \`NODE_ENV=production\` - Production mode (from render.yaml)
- \`RUST_LOG=info\` - Rust logging level (from render.yaml)

Optional variables you can add in Render dashboard:
- \`RUST_LOG=debug\` - Enhanced logging

---
`

  if (existsSync(readmePath)) {
    const readmeContent = readFileSync(readmePath, 'utf-8')
    if (!readmeContent.includes('Deploy to Render')) {
      writeFileSync(readmePath, readmeContent + renderReadmeSection)
      logSuccess('Updated README.md with Render deployment instructions')
    }
  }
  else {
    const defaultReadme = `# My rari App

A high-performance React Server Components application powered by rari.
${renderReadmeSection}
## Getting Started

\`\`\`bash
npm install
npm start
\`\`\`

Visit [http://localhost:3000](http://localhost:3000) to see your app.
`
    writeFileSync(readmePath, defaultReadme)
    logSuccess('Created README.md with Render deployment instructions')
  }
}

function printRenderSuccessMessage() {
  console.warn('')
  logSuccess('Render deployment setup complete! 🎉')
  console.warn('')
  logInfo('Next steps:')
  console.warn(`  1. ${styleText('cyan', 'git add .')}`)
  console.warn(`  2. ${styleText('cyan', 'git commit -m "Add Render deployment"')}`)
  console.warn(`  3. ${styleText('cyan', 'git push origin main')}`)
  console.warn(`  4. Go to ${styleText('cyan', 'https://render.com')} and create a Web Service`)
  console.warn('')
  logInfo('Your rari app will automatically:')
  console.warn('  ✅ Detect Render environment')
  console.warn('  ✅ Bind to 0.0.0.0 (Render requirement)')
  console.warn('  ✅ Use Render\'s PORT environment variable')
  console.warn('  ✅ Run in production mode')
  console.warn('  ✅ Download platform-specific rari binary')
  console.warn('')
  logSuccess('Ready for deployment! 🚀')
}
