import process from 'node:process'
import { getDeploymentConfig } from '@rari/cli'
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
})
