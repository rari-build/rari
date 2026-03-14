import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { resolve } from 'node:path'
import process from 'node:process'
import { describe, expect, it } from 'vite-plus/test'

const CLI_PATH = resolve(process.cwd(), 'packages/rari/dist/cli.mjs')
const TIMEOUT = 10000

function runCLI(args: string[], options: { timeout?: number, env?: Record<string, string> } = {}): Promise<{
  code: number | null
  stdout: string
  stderr: string
}> {
  return new Promise((resolve, reject) => {
    const child = spawn('node', [CLI_PATH, ...args], {
      cwd: process.cwd(),
      env: { ...process.env, ...options.env },
    })

    let stdout = ''
    let stderr = ''

    child.stdout?.on('data', (data) => {
      stdout += data.toString()
    })

    child.stderr?.on('data', (data) => {
      stderr += data.toString()
    })

    const timeout = setTimeout(() => {
      child.kill('SIGTERM')
      reject(new Error(`CLI command timed out after ${options.timeout || TIMEOUT}ms`))
    }, options.timeout || TIMEOUT)

    child.on('exit', (code) => {
      clearTimeout(timeout)
      resolve({ code, stdout, stderr })
    })

    child.on('error', (error) => {
      clearTimeout(timeout)
      reject(error)
    })
  })
}

describe('CLI Integration Tests', () => {
  it('should have CLI built', () => {
    expect(existsSync(CLI_PATH)).toBe(true)
  })

  describe('help command', () => {
    it('should display help with no arguments', async () => {
      const { code, stderr } = await runCLI([])

      expect(code).toBe(0)
      expect(stderr).toContain('rari CLI')
      expect(stderr).toContain('Usage:')
      expect(stderr).toContain('rari dev')
      expect(stderr).toContain('rari build')
      expect(stderr).toContain('rari start')
      expect(stderr).toContain('rari deploy')
    }, TIMEOUT)

    it('should display help with help command', async () => {
      const { code, stderr } = await runCLI(['help'])

      expect(code).toBe(0)
      expect(stderr).toContain('rari CLI')
      expect(stderr).toContain('Usage:')
    }, TIMEOUT)

    it('should display help with --help flag', async () => {
      const { code, stderr } = await runCLI(['--help'])

      expect(code).toBe(0)
      expect(stderr).toContain('rari CLI')
    }, TIMEOUT)

    it('should display help with -h flag', async () => {
      const { code, stderr } = await runCLI(['-h'])

      expect(code).toBe(0)
      expect(stderr).toContain('rari CLI')
    }, TIMEOUT)

    it('should show environment variables section', async () => {
      const { stderr } = await runCLI(['help'])

      expect(stderr).toContain('Environment Variables:')
      expect(stderr).toContain('PORT')
      expect(stderr).toContain('NODE_ENV')
    }, TIMEOUT)

    it('should show vite-plus commands in examples', async () => {
      const { stderr } = await runCLI(['help'])

      expect(stderr).toContain('Examples:')
      expect(stderr).toContain('development server')
    }, TIMEOUT)

    it('should show deployment options', async () => {
      const { stderr } = await runCLI(['help'])

      expect(stderr).toContain('Deployment:')
      expect(stderr).toContain('railway')
      expect(stderr).toContain('render')
    }, TIMEOUT)
  })

  describe('unknown command', () => {
    it('should error on unknown command', async () => {
      const { code, stderr } = await runCLI(['unknown-command'])

      expect(code).toBe(1)
      expect(stderr).toContain('Unknown command')
      expect(stderr).toContain('rari help')
    }, TIMEOUT)

    it('should suggest help command', async () => {
      const { stderr } = await runCLI(['invalid'])

      expect(stderr).toContain('rari help')
    }, TIMEOUT)
  })

  describe('deploy command', () => {
    it('should error on deploy without target', async () => {
      const { code, stderr } = await runCLI(['deploy'])

      expect(code).toBe(1)
      expect(stderr).toContain('Unknown deployment target')
      expect(stderr).toContain('railway')
      expect(stderr).toContain('render')
    }, TIMEOUT)

    it('should error on unknown deployment target', async () => {
      const { code, stderr } = await runCLI(['deploy', 'unknown'])

      expect(code).toBe(1)
      expect(stderr).toContain('Unknown deployment target')
    }, TIMEOUT)
  })

  describe('environment variable handling', () => {
    it('should work with custom PORT', async () => {
      const { stderr } = await runCLI(['help'], {
        env: { PORT: '8080' },
      })

      expect(stderr).toContain('rari CLI')
    }, TIMEOUT)

    it('should work with NODE_ENV', async () => {
      const { stderr } = await runCLI(['help'], {
        env: { NODE_ENV: 'development' },
      })

      expect(stderr).toContain('rari CLI')
    }, TIMEOUT)
  })

  describe('platform detection', () => {
    it('should work without platform variables', async () => {
      // Create env without platform variables
      const env = Object.fromEntries(
        Object.entries(process.env).filter(([key]) =>
          key !== 'RAILWAY_ENVIRONMENT' && key !== 'RENDER',
        ),
      ) as Record<string, string>

      const { code, stderr } = await runCLI(['help'], { env })

      expect(code).toBe(0)
      expect(stderr).toContain('rari CLI')
    }, TIMEOUT)

    it('should work with Railway environment', async () => {
      const { code, stderr } = await runCLI(['help'], {
        env: { RAILWAY_ENVIRONMENT: 'production' },
      })

      expect(code).toBe(0)
      expect(stderr).toContain('rari CLI')
    }, TIMEOUT)

    it('should work with Render environment', async () => {
      const { code, stderr } = await runCLI(['help'], {
        env: { RENDER: 'true' },
      })

      expect(code).toBe(0)
      expect(stderr).toContain('rari CLI')
    }, TIMEOUT)
  })

  describe('vite-plus migration verification', () => {
    it('should recognize dev command (vite-plus)', async () => {
      const { stderr } = await runCLI(['help'])

      expect(stderr).toContain('rari dev')
      expect(stderr).toContain('Vite')
    }, TIMEOUT)

    it('should recognize build command (vite-plus)', async () => {
      const { stderr } = await runCLI(['help'])

      expect(stderr).toContain('rari build')
      expect(stderr).toContain('production')
    }, TIMEOUT)

    it('should show correct command structure', async () => {
      const { stderr } = await runCLI(['help'])

      expect(stderr).toContain('rari dev')
      expect(stderr).toContain('rari build')
      expect(stderr).toContain('rari start')
      expect(stderr).toContain('rari deploy')
    }, TIMEOUT)
  })
})
