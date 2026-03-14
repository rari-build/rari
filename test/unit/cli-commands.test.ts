import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { resolve } from 'node:path'
import process from 'node:process'
import { describe, expect, it } from 'vite-plus/test'

const CLI_PATH = resolve(process.cwd(), 'packages/rari/dist/cli.mjs')

function runCLI(args: string[], env: Record<string, string> = {}): Promise<{ code: number, stdout: string, stderr: string }> {
  return new Promise((resolve) => {
    const child = spawn('node', [CLI_PATH, ...args], {
      env: { ...process.env, ...env },
      cwd: process.cwd(),
    })

    let stdout = ''
    let stderr = ''

    child.stdout?.on('data', (data) => {
      stdout += data.toString()
    })

    child.stderr?.on('data', (data) => {
      stderr += data.toString()
    })

    child.on('close', (code) => {
      resolve({ code: code || 0, stdout, stderr })
    })

    child.on('error', () => {
      resolve({ code: 1, stdout, stderr })
    })

    setTimeout(() => {
      child.kill()
      resolve({ code: -1, stdout, stderr })
    }, 2000)
  })
}

describe('cli commands', () => {
  it('should have CLI binary built', () => {
    expect(existsSync(CLI_PATH)).toBe(true)
  })

  describe('help command', () => {
    it('should display help text', async () => {
      const { code, stderr } = await runCLI(['help'])
      expect(code).toBe(0)
      expect(stderr).toContain('rari')
    })
  })

  describe('environment variable handling', () => {
    it('should accept PORT environment variable', async () => {
      const { stderr } = await runCLI(['help'], { PORT: '8080' })
      expect(stderr).toContain('rari')
    })

    it('should accept NODE_ENV environment variable', async () => {
      const { stderr } = await runCLI(['help'], { NODE_ENV: 'development' })
      expect(stderr).toContain('rari')
    })

    it('should accept RAILWAY_ENVIRONMENT', async () => {
      const { stderr } = await runCLI(['help'], { RAILWAY_ENVIRONMENT: 'production' })
      expect(stderr).toContain('rari')
    })

    it('should accept RENDER environment', async () => {
      const { stderr } = await runCLI(['help'], { RENDER: 'true' })
      expect(stderr).toContain('rari')
    })
  })

  describe('command availability', () => {
    const commands = ['dev', 'build', 'start', 'deploy', 'help']

    commands.forEach((cmd) => {
      it(`should recognize ${cmd} command in help`, async () => {
        const { stderr } = await runCLI(['help'])
        expect(stderr).toContain(cmd)
      })
    })
  })
})
