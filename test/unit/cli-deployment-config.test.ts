import process from 'node:process'
import { getDeploymentConfig, isFlyEnvironment } from '@rari/cli'
import { afterEach, beforeEach, describe, expect, it } from 'vite-plus/test'

describe('cli deployment config', () => {
  const originalEnv = process.env

  beforeEach(() => {
    process.env = {}
  })

  afterEach(() => {
    process.env = originalEnv
  })

  it('binds to RARI_HOST when set', () => {
    process.env.RARI_HOST = '0.0.0.0'
    expect(getDeploymentConfig().host).toBe('0.0.0.0')

    process.env.RARI_HOST = '192.168.1.10'
    process.env.RENDER = 'true'
    expect(getDeploymentConfig().host).toBe('192.168.1.10')
  })

  it('binds to RARI_HOST over the Fly default', () => {
    process.env.FLY_APP_NAME = 'my-app'
    process.env.RARI_HOST = '192.168.1.10'
    expect(getDeploymentConfig().host).toBe('192.168.1.10')
  })

  it('falls back to the default host when RARI_HOST is empty', () => {
    process.env.RARI_HOST = ''
    expect(getDeploymentConfig().host).toBe('127.0.0.1')

    process.env.RENDER = 'true'
    expect(getDeploymentConfig().host).toBe('0.0.0.0')
  })

  it('falls back to the default host when RARI_HOST is unset', () => {
    expect(getDeploymentConfig().host).toBe('127.0.0.1')

    process.env.RAILWAY_ENVIRONMENT = 'production'
    expect(getDeploymentConfig().host).toBe('0.0.0.0')
  })

  it('defaults to 0.0.0.0 on Fly.io', () => {
    expect(getDeploymentConfig().host).toBe('127.0.0.1')

    process.env.FLY_APP_NAME = 'my-app'
    expect(getDeploymentConfig().host).toBe('0.0.0.0')
  })

  it('defaults to 0.0.0.0 on Fly.io when RARI_HOST is empty', () => {
    process.env.FLY_MACHINE_ID = '148ed599e12e08'
    process.env.RARI_HOST = ''
    expect(getDeploymentConfig().host).toBe('0.0.0.0')
  })

  describe('isFlyEnvironment', () => {
    it('detects Fly from FLY_APP_NAME', () => {
      process.env.FLY_APP_NAME = 'my-app'
      expect(isFlyEnvironment()).toBe(true)
    })

    it('detects Fly from FLY_MACHINE_ID', () => {
      process.env.FLY_MACHINE_ID = '148ed599e12e08'
      expect(isFlyEnvironment()).toBe(true)
    })

    it('detects Fly from FLY_ALLOC_ID', () => {
      process.env.FLY_ALLOC_ID = 'alloc-id'
      expect(isFlyEnvironment()).toBe(true)
    })

    it('ignores empty Fly variables', () => {
      process.env.FLY_APP_NAME = ''
      process.env.FLY_MACHINE_ID = ''
      process.env.FLY_ALLOC_ID = ''
      expect(isFlyEnvironment()).toBe(false)
    })

    it('returns false when no Fly variables are set', () => {
      expect(isFlyEnvironment()).toBe(false)
    })
  })
})
