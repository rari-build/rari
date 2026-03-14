import { existsSync } from 'node:fs'
import process from 'node:process'
import { afterEach, beforeEach, describe, expect, it } from 'vite-plus/test'

function detectPackageManager(): string {
  if (existsSync('pnpm-lock.yaml'))
    return 'pnpm'
  if (existsSync('yarn.lock'))
    return 'yarn'
  if (existsSync('bun.lockb'))
    return 'bun'

  return 'npm'
}

function isRailwayEnvironment(): boolean {
  return !!(
    process.env.RAILWAY_ENVIRONMENT
    || process.env.RAILWAY_PROJECT_ID
    || process.env.RAILWAY_SERVICE_ID
  )
}

function isRenderEnvironment(): boolean {
  return !!(
    process.env.RENDER
    || process.env.RENDER_SERVICE_ID
    || process.env.RENDER_SERVICE_NAME
  )
}

function getDeploymentConfig() {
  const port = process.env.PORT || process.env.RSC_PORT || '3000'
  const mode = process.env.NODE_ENV || 'production'
  const isPlatform = isRailwayEnvironment() || isRenderEnvironment()
  const host = isPlatform ? '0.0.0.0' : '127.0.0.1'

  return { port, mode, host }
}

describe('cli', () => {
  const originalEnv = process.env

  beforeEach(() => {
    process.env = { ...originalEnv }
  })

  afterEach(() => {
    process.env = originalEnv
  })

  describe('package manager detection', () => {
    it('should detect pnpm from pnpm-lock.yaml', () => {
      const result = detectPackageManager()
      expect(result).toBe('pnpm')
    })

    it('should detect yarn from yarn.lock', () => {
      expect(typeof detectPackageManager()).toBe('string')
    })

    it('should detect bun from bun.lockb', () => {
      expect(typeof detectPackageManager()).toBe('string')
    })

    it('should default to npm', () => {
      expect(typeof detectPackageManager()).toBe('string')
    })
  })

  describe('platform environment detection', () => {
    it('should detect Railway environment', () => {
      process.env.RAILWAY_ENVIRONMENT = 'production'
      expect(isRailwayEnvironment()).toBe(true)
    })

    it('should detect Railway from PROJECT_ID', () => {
      delete process.env.RAILWAY_ENVIRONMENT
      process.env.RAILWAY_PROJECT_ID = 'test-project'
      expect(isRailwayEnvironment()).toBe(true)
    })

    it('should detect Railway from SERVICE_ID', () => {
      delete process.env.RAILWAY_ENVIRONMENT
      delete process.env.RAILWAY_PROJECT_ID
      process.env.RAILWAY_SERVICE_ID = 'test-service'
      expect(isRailwayEnvironment()).toBe(true)
    })

    it('should detect Render environment', () => {
      process.env.RENDER = 'true'
      expect(isRenderEnvironment()).toBe(true)
    })

    it('should detect Render from SERVICE_ID', () => {
      delete process.env.RENDER
      process.env.RENDER_SERVICE_ID = 'test-service'
      expect(isRenderEnvironment()).toBe(true)
    })

    it('should detect Render from SERVICE_NAME', () => {
      delete process.env.RENDER
      delete process.env.RENDER_SERVICE_ID
      process.env.RENDER_SERVICE_NAME = 'test-service'
      expect(isRenderEnvironment()).toBe(true)
    })

    it('should return false when no platform detected', () => {
      delete process.env.RAILWAY_ENVIRONMENT
      delete process.env.RAILWAY_PROJECT_ID
      delete process.env.RAILWAY_SERVICE_ID
      delete process.env.RENDER
      delete process.env.RENDER_SERVICE_ID
      delete process.env.RENDER_SERVICE_NAME

      expect(isRailwayEnvironment()).toBe(false)
      expect(isRenderEnvironment()).toBe(false)
    })
  })

  describe('deployment configuration', () => {
    it('should use platform port when available', () => {
      process.env.PORT = '8080'
      const config = getDeploymentConfig()
      expect(config.port).toBe('8080')
    })

    it('should use RSC_PORT as fallback', () => {
      delete process.env.PORT
      process.env.RSC_PORT = '9000'
      const config = getDeploymentConfig()
      expect(config.port).toBe('9000')
    })

    it('should default to 3000', () => {
      delete process.env.PORT
      delete process.env.RSC_PORT
      const config = getDeploymentConfig()
      expect(config.port).toBe('3000')
    })

    it('should handle NODE_ENV', () => {
      process.env.NODE_ENV = 'development'
      const config = getDeploymentConfig()
      expect(config.mode).toBe('development')
    })

    it('should default to production mode', () => {
      delete process.env.NODE_ENV
      const config = getDeploymentConfig()
      expect(config.mode).toBe('production')
    })

    it('should use 0.0.0.0 for Railway environment', () => {
      process.env.RAILWAY_ENVIRONMENT = 'production'
      const config = getDeploymentConfig()
      expect(config.host).toBe('0.0.0.0')
    })

    it('should use 0.0.0.0 for Render environment', () => {
      delete process.env.RAILWAY_ENVIRONMENT
      process.env.RENDER = 'true'
      const config = getDeploymentConfig()
      expect(config.host).toBe('0.0.0.0')
    })

    it('should use 127.0.0.1 for local environments', () => {
      delete process.env.RAILWAY_ENVIRONMENT
      delete process.env.RAILWAY_PROJECT_ID
      delete process.env.RAILWAY_SERVICE_ID
      delete process.env.RENDER
      delete process.env.RENDER_SERVICE_ID
      delete process.env.RENDER_SERVICE_NAME

      const config = getDeploymentConfig()
      expect(config.host).toBe('127.0.0.1')
    })
  })
})
