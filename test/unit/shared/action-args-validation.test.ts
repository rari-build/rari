import { describe, expect, it } from 'vite-plus/test'
import {
  developmentValidationConfig,
  productionValidationConfig,
  validateActionArgs,
  validateFormData,
} from '../../support/action-args-validation'

describe('validateActionArgs', () => {
  const config = productionValidationConfig()

  it('removes dangerous prototype keys', () => {
    const sanitized = validateActionArgs([{
      __proto__: { isAdmin: true },
      username: 'test',
    }], config)

    expect(sanitized[0]).toEqual({ username: 'test' })
  })

  it('enforces nesting depth', () => {
    const valid = [{ level1: { level2: { level3: 'ok' } } }]
    expect(() => validateActionArgs(valid, { ...config, maxDepth: 3 })).not.toThrow()

    const invalid = [{ level1: { level2: { level3: { level4: 'too deep' } } } }]
    expect(() => validateActionArgs(invalid, { ...config, maxDepth: 3 }))
      .toThrow(/nesting depth/)
  })

  it('rejects wide nested array DoS payloads', () => {
    const config = {
      ...productionValidationConfig(),
      maxDepth: 10,
      maxTotalElements: 10_000,
      maxArrayLength: 1_000,
    }

    const outerArray = Array.from({ length: 20 }, () => Array.from({ length: 600 }).fill(1))
    expect(() => validateActionArgs([{ data: outerArray }], config))
      .toThrow(/Maximum array nesting exceeded|12000 > 10000/)
  })

  it('detects forked array trees', () => {
    const config = {
      ...productionValidationConfig(),
      maxDepth: 5,
      maxTotalElements: 1_000,
      maxArrayLength: 500,
    }

    expect(() => validateActionArgs([{ data: [Array.from({ length: 500 }).fill(1)] }], config))
      .not
      .toThrow()

    expect(() => validateActionArgs([
      { data: [Array.from({ length: 500 }).fill(1), Array.from({ length: 500 }).fill(2)] },
    ], config)).toThrow(/Maximum array nesting exceeded/)
  })

  it('preserves FormData arguments', () => {
    const formData = new FormData()
    formData.set('text', 'hello')
    const sanitized = validateActionArgs([formData], config)
    expect(sanitized[0]).toBe(formData)
  })

  it('preserves bigint and Date arguments without coercing them to objects', () => {
    const when = new Date('2026-01-01T00:00:00.000Z')
    const sanitized = validateActionArgs([1n, when], config)

    expect(sanitized[0]).toBe(1n)
    expect(sanitized[1]).toBe(when)
  })

  it('rejects Map and Set arguments', () => {
    expect(() => validateActionArgs([new Map([['a', 1]])], config)).toThrow(/Map is not supported/)
    expect(() => validateActionArgs([new Set([1])], config)).toThrow(/Set is not supported/)
  })
})

describe('validateFormData', () => {
  it('limits user field length but skips Flight metadata keys', () => {
    const config = developmentValidationConfig()
    const formData = new FormData()
    formData.set('$ACTION_REF_1', 'x'.repeat(20_000))
    formData.set('text', 'ok')

    expect(() => validateFormData(formData, config)).not.toThrow()

    formData.set('text', 'x'.repeat(config.maxStringLength + 1))
    expect(() => validateFormData(formData, config)).toThrow(/Form field "text" too long/)
  })
})
