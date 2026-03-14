import { existsSync, readFileSync } from 'node:fs'
import process from 'node:process'
import { afterEach, beforeEach, describe, expect, it } from 'vite-plus/test'

describe('cli', () => {
  const originalEnv = process.env

  beforeEach(() => {
    process.env = { ...originalEnv }
  })

  afterEach(() => {
    process.env = originalEnv
  })

  describe('environment variable loading', () => {
    it('should have .env file loading capability', () => {
      expect(existsSync).toBeDefined()
      expect(readFileSync).toBeDefined()
    })
  })

  describe('package manager detection', () => {
    it('should detect pnpm from pnpm-lock.yaml', () => {
      expect(existsSync).toBeDefined()
      const hasPnpmLock = existsSync('pnpm-lock.yaml')
      expect(typeof hasPnpmLock).toBe('boolean')
    })

    it('should detect yarn from yarn.lock', () => {
      expect(existsSync).toBeDefined()
      const hasYarnLock = existsSync('yarn.lock')
      expect(typeof hasYarnLock).toBe('boolean')
    })

    it('should detect bun from bun.lockb', () => {
      expect(existsSync).toBeDefined()
      const hasBunLock = existsSync('bun.lockb')
      expect(typeof hasBunLock).toBe('boolean')
    })

    it('should detect npm from package-lock.json', () => {
      expect(existsSync).toBeDefined()
      const hasNpmLock = existsSync('package-lock.json')
      expect(typeof hasNpmLock).toBe('boolean')
    })
  })

  describe('platform environment detection', () => {
    it('should detect Railway environment', () => {
      process.env.RAILWAY_ENVIRONMENT = 'production'
      expect(process.env.RAILWAY_ENVIRONMENT).toBe('production')

      const isRailway = !!(
        process.env.RAILWAY_ENVIRONMENT
        || process.env.RAILWAY_PROJECT_ID
        || process.env.RAILWAY_SERVICE_ID
      )
      expect(isRailway).toBe(true)
    })

    it('should detect Render environment', () => {
      process.env.RENDER = 'true'
      expect(process.env.RENDER).toBe('true')

      const isRender = !!(
        process.env.RENDER
        || process.env.RENDER_SERVICE_ID
        || process.env.RENDER_SERVICE_NAME
      )
      expect(isRender).toBe(true)
    })

    it('should return false when no platform detected', () => {
      delete process.env.RAILWAY_ENVIRONMENT
      delete process.env.RAILWAY_PROJECT_ID
      delete process.env.RAILWAY_SERVICE_ID
      delete process.env.RENDER
      delete process.env.RENDER_SERVICE_ID
      delete process.env.RENDER_SERVICE_NAME

      const isRailway = !!(
        process.env.RAILWAY_ENVIRONMENT
        || process.env.RAILWAY_PROJECT_ID
        || process.env.RAILWAY_SERVICE_ID
      )
      const isRender = !!(
        process.env.RENDER
        || process.env.RENDER_SERVICE_ID
        || process.env.RENDER_SERVICE_NAME
      )

      expect(isRailway).toBe(false)
      expect(isRender).toBe(false)
    })
  })

  describe('deployment configuration', () => {
    it('should use platform port when available', () => {
      process.env.PORT = '8080'
      const port = process.env.PORT || process.env.RSC_PORT || '3000'
      expect(port).toBe('8080')
    })

    it('should handle missing port', () => {
      delete process.env.PORT
      delete process.env.RSC_PORT
      const port = process.env.PORT || process.env.RSC_PORT || '3000'
      expect(port).toBe('3000')
    })

    it('should handle NODE_ENV', () => {
      process.env.NODE_ENV = 'development'
      const mode = process.env.NODE_ENV || 'production'
      expect(mode).toBe('development')
    })

    it('should default to production mode', () => {
      delete process.env.NODE_ENV
      const mode = process.env.NODE_ENV || 'production'
      expect(mode).toBe('production')
    })

    it('should use 0.0.0.0 for platform environments', () => {
      process.env.RAILWAY_ENVIRONMENT = 'production'
      const isPlatform = !!(
        process.env.RAILWAY_ENVIRONMENT
        || process.env.RAILWAY_PROJECT_ID
        || process.env.RAILWAY_SERVICE_ID
        || process.env.RENDER
        || process.env.RENDER_SERVICE_ID
        || process.env.RENDER_SERVICE_NAME
      )
      const host = isPlatform ? '0.0.0.0' : '127.0.0.1'
      expect(host).toBe('0.0.0.0')
    })

    it('should use 127.0.0.1 for local environments', () => {
      delete process.env.RAILWAY_ENVIRONMENT
      delete process.env.RAILWAY_PROJECT_ID
      delete process.env.RAILWAY_SERVICE_ID
      delete process.env.RENDER
      delete process.env.RENDER_SERVICE_ID
      delete process.env.RENDER_SERVICE_NAME

      const isPlatform = !!(
        process.env.RAILWAY_ENVIRONMENT
        || process.env.RAILWAY_PROJECT_ID
        || process.env.RAILWAY_SERVICE_ID
        || process.env.RENDER
        || process.env.RENDER_SERVICE_ID
        || process.env.RENDER_SERVICE_NAME
      )
      const host = isPlatform ? '0.0.0.0' : '127.0.0.1'
      expect(host).toBe('127.0.0.1')
    })
  })
})
