import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import colors from 'picocolors'

function logInfo(message: string) {
  console.warn(`${colors.blue('info')} ${message}`)
}

function logSuccess(message: string) {
  console.warn(`${colors.green('✓')} ${message}`)
}

function logError(message: string) {
  console.error(`${colors.red('✗')} ${message}`)
}

function logWarning(message: string) {
  console.warn(`${colors.yellow('⚠')} ${message}`)
}

export async function createRenderDeployment() {
  const cwd = process.cwd()

  logInfo('Creating Render deployment configuration...')

  const packageJsonPath = join(cwd, 'package.json')
  if (!existsSync(packageJsonPath)) {
    logError('No package.json found. Please run this command from your project root.')
    process.exit(1)
  }

  try {
    const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'))

    packageJson.scripts = packageJson.scripts || {}

    if (packageJson.scripts.start && packageJson.scripts.start !== 'rari start') {
      logWarning(`Existing start script found: "${packageJson.scripts.start}"`)
      logWarning('Backing up to start:original and replacing with "rari start"')
      packageJson.scripts['start:original'] = packageJson.scripts.start
    }

    packageJson.scripts.start = 'rari start'
    packageJson.scripts['start:local'] = 'rari start'
    packageJson.scripts['deploy:render'] = 'echo "Push to GitHub and connect to Render to deploy"'

    packageJson.engines = packageJson.engines || {}
    if (!packageJson.engines.node) {
      packageJson.engines.node = '>=20.0.0'
    }

    if (!packageJson.dependencies || !packageJson.dependencies.rari) {
      logInfo('Adding rari dependency...')
      packageJson.dependencies = packageJson.dependencies || {}
      packageJson.dependencies.rari = '^0.1.0'
    }

    writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`)
    logSuccess('Updated package.json for Render deployment')
  }
  catch (error) {
    logError(`Failed to update package.json: ${error instanceof Error ? error.message : 'Unknown error'}`)
    process.exit(1)
  }

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

  const renderYamlPath = join(cwd, 'render.yaml')
  if (existsSync(renderYamlPath)) {
    logWarning('render.yaml already exists, backing up to render.yaml.backup')
    const existingConfig = readFileSync(renderYamlPath, 'utf-8')
    writeFileSync(join(cwd, 'render.yaml.backup'), existingConfig)
  }

  writeFileSync(renderYamlPath, renderConfig)
  logSuccess('Created render.yaml configuration')

  const gitignorePath = join(cwd, '.gitignore')
  const renderGitignoreEntries = [
    '',
    '# Render',
    '.render/',
    '',
  ].join('\n')

  if (existsSync(gitignorePath)) {
    const gitignoreContent = readFileSync(gitignorePath, 'utf-8')
    if (!gitignoreContent.includes('.render/')) {
      writeFileSync(gitignorePath, gitignoreContent + renderGitignoreEntries)
      logSuccess('Updated .gitignore with Render entries')
    }
  }
  else {
    const defaultGitignore = `# Dependencies
node_modules/
.pnpm-store/

# Build outputs
.rari/
dist/

# Environment variables
.env
.env.local
.env.production

# Render
.render/

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
    logSuccess('Created .gitignore with Render entries')
  }

  const readmePath = join(cwd, 'README.md')
  const renderReadmeSection = `
## 🎨 Deploy to Render

This Rari application is configured for Render deployment.

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
    const defaultReadme = `# My Rari App

A high-performance React Server Components application powered by Rari.
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

  console.warn('')
  logSuccess('Render deployment setup complete! 🎉')
  console.warn('')
  logInfo('Next steps:')
  console.warn(`  1. ${colors.cyan('git add .')}`)
  console.warn(`  2. ${colors.cyan('git commit -m "Add Render deployment"')}`)
  console.warn(`  3. ${colors.cyan('git push origin main')}`)
  console.warn(`  4. Go to ${colors.cyan('https://render.com')} and create a Web Service`)
  console.warn('')
  logInfo('Your Rari app will automatically:')
  console.warn('  ✅ Detect Render environment')
  console.warn('  ✅ Bind to 0.0.0.0 (Render requirement)')
  console.warn('  ✅ Use Render\'s PORT environment variable')
  console.warn('  ✅ Run in production mode')
  console.warn('  ✅ Download platform-specific Rari binary')
  console.warn('')
  logSuccess('Ready for deployment! 🚀')
}
