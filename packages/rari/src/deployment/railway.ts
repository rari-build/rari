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
      logWarning(`Existing start script found: "${packageJson.scripts.start}"`)
      logWarning('Backing up to start:original and replacing with "rari start"')
      packageJson.scripts['start:original'] = packageJson.scripts.start
    }

    packageJson.scripts.start = 'rari start'
    packageJson.scripts['start:local'] = 'rari start'
    packageJson.scripts['deploy:railway'] = 'echo "Push to GitHub and connect to Railway to deploy"'

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
    logSuccess('Updated package.json for Railway deployment')
  }
  catch (error) {
    logError(`Failed to update package.json: ${error instanceof Error ? error.message : 'Unknown error'}`)
    process.exit(1)
  }

  const railwayConfig = {
    $schema: 'https://railway.app/railway.schema.json',
    build: {
      builder: 'NIXPACKS',
    },
    deploy: {
      startCommand: 'npm start',
      healthcheckPath: '/',
      healthcheckTimeout: 300,
      restartPolicyType: 'ALWAYS',
    },
    environments: {
      production: {
        variables: {
          NODE_ENV: 'production',
          RUST_LOG: 'info',
        },
      },
    },
  }

  const railwayJsonPath = join(cwd, 'railway.json')
  if (existsSync(railwayJsonPath)) {
    logWarning('railway.json already exists, backing up to railway.json.backup')
    const existingConfig = readFileSync(railwayJsonPath, 'utf-8')
    writeFileSync(join(cwd, 'railway.json.backup'), existingConfig)
  }

  writeFileSync(railwayJsonPath, `${JSON.stringify(railwayConfig, null, 2)}\n`)
  logSuccess('Created railway.json configuration')

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
.rari/
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

This Rari application is configured for Railway deployment.

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
    const defaultReadme = `# My Rari App

A high-performance React Server Components application powered by Rari.
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
  console.warn(`  1. ${colors.cyan('git add .')}`)
  console.warn(`  2. ${colors.cyan('git commit -m "Add Railway deployment"')}`)
  console.warn(`  3. ${colors.cyan('git push origin main')}`)
  console.warn(`  4. Go to ${colors.cyan('https://railway.app')} and deploy from GitHub`)
  console.warn('')
  logInfo('Your Rari app will automatically:')
  console.warn('  ✅ Detect Railway environment')
  console.warn('  ✅ Bind to 0.0.0.0 (Railway requirement)')
  console.warn('  ✅ Use Railway\'s PORT environment variable')
  console.warn('  ✅ Run in production mode')
  console.warn('  ✅ Download platform-specific Rari binary')
  console.warn('')
  logSuccess('Ready for deployment! 🚀')
}
