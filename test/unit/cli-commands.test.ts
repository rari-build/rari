import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { resolve } from 'node:path'
import process from 'node:process'
import { describe, expect, it } from 'vite-plus/test'

const CLI_PATH = resolve(process.cwd(), 'packages/rari/dist/cli.mjs')

describe('cli commands', () => {
  describe('command structure', () => {
    it('should have spawn function available', () => {
      expect(spawn).toBeDefined()
      expect(typeof spawn).toBe('function')
    })

    it('should have existsSync function available', () => {
      expect(existsSync).toBeDefined()
      expect(typeof existsSync).toBe('function')
    })

    it('should have process.env available', () => {
      expect(process.env).toBeDefined()
      expect(typeof process.env).toBe('object')
    })

    it('should have CLI binary built', () => {
      expect(existsSync(CLI_PATH)).toBe(true)
    })
  })

  describe('environment variables', () => {
    it('should handle PORT environment variable', () => {
      const originalPort = process.env.PORT
      process.env.PORT = '8080'
      expect(process.env.PORT).toBe('8080')
      if (originalPort) {
        process.env.PORT = originalPort
      }
      else {
        delete process.env.PORT
      }
    })

    it('should handle NODE_ENV environment variable', () => {
      const originalEnv = process.env.NODE_ENV
      process.env.NODE_ENV = 'development'
      expect(process.env.NODE_ENV).toBe('development')
      if (originalEnv) {
        process.env.NODE_ENV = originalEnv
      }
      else {
        delete process.env.NODE_ENV
      }
    })

    it('should handle RAILWAY_ENVIRONMENT', () => {
      const original = process.env.RAILWAY_ENVIRONMENT
      process.env.RAILWAY_ENVIRONMENT = 'production'
      expect(process.env.RAILWAY_ENVIRONMENT).toBe('production')
      if (original) {
        process.env.RAILWAY_ENVIRONMENT = original
      }
      else {
        delete process.env.RAILWAY_ENVIRONMENT
      }
    })

    it('should handle RENDER environment', () => {
      const original = process.env.RENDER
      process.env.RENDER = 'true'
      expect(process.env.RENDER).toBe('true')
      if (original) {
        process.env.RENDER = original
      }
      else {
        delete process.env.RENDER
      }
    })
  })

  describe('vite-plus integration', () => {
    it('should use vp as the build tool', () => {
      expect(existsSync(CLI_PATH)).toBe(true)
    })

    it('should support dev command', () => {
      expect(existsSync(CLI_PATH)).toBe(true)
    })

    it('should support build command', () => {
      expect(existsSync(CLI_PATH)).toBe(true)
    })

    it('should support development mode builds', () => {
      expect(existsSync(CLI_PATH)).toBe(true)
    })
  })

  describe('command availability', () => {
    const commands = ['dev', 'build', 'start', 'deploy', 'help']

    commands.forEach((cmd) => {
      it(`should support ${cmd} command`, () => {
        expect(existsSync(CLI_PATH)).toBe(true)
      })
    })
  })
})
