/// <reference path="../../types.d.ts" />

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
;(function initActionArgsValidationCore(g: GlobalThis) {
  if (g.__RARI_ACTION_ARGS_VALIDATION__ !== undefined) return

  const MAX_BOUND_ARGS = 1000
  const MAX_FORM_FIELDS = 1000
  const MAX_BIGINT_DIGITS = 300

  function productionValidationConfig(): ActionValidationConfig {
    return {
      maxDepth: 10,
      maxStringLength: 10_000,
      maxArrayLength: 1_000,
      maxObjectKeys: 100,
      maxTotalElements: 1_000_000,
    }
  }

  function developmentValidationConfig(): ActionValidationConfig {
    return {
      maxDepth: 20,
      maxStringLength: 50_000,
      maxArrayLength: 5_000,
      maxObjectKeys: 500,
      maxTotalElements: 5_000_000,
    }
  }

  function isDangerousActionProperty(key: string): boolean {
    return (
      key === '__proto__' ||
      key === 'constructor' ||
      key === 'prototype' ||
      key === '__defineGetter__' ||
      key === '__defineSetter__' ||
      key === '__lookupGetter__' ||
      key === '__lookupSetter__'
    )
  }

  function isFlightFormMetadataKey(key: string): boolean {
    return key.startsWith('$ACTION')
  }

  function isOpaqueActionArg(value: unknown): boolean {
    return (
      value instanceof FormData ||
      (typeof Blob !== 'undefined' && value instanceof Blob) ||
      (typeof File !== 'undefined' && value instanceof File)
    )
  }

  function estimatedDigitCount(absValue: number): number {
    if (absValue === 0) return 1

    return Math.floor(Math.log10(absValue)) + 1
  }

  interface ActionValidationContext {
    totalElements: number
    hasFork: boolean
  }

  function bumpActionValidationCount(
    // oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- ctx is intentionally mutated in place as a running counter
    ctx: ActionValidationContext,
    count: number,
    config: ActionValidationConfig,
  ): void {
    ctx.totalElements += count
    if (ctx.hasFork && ctx.totalElements > config.maxTotalElements) {
      throw new TypeError(
        `Maximum array nesting exceeded: ${ctx.totalElements} > ${config.maxTotalElements}. ` +
          'Large nested arrays can be dangerous. Try adding intermediate objects.',
      )
    }
  }

  function validateActionValue(
    value: unknown,
    config: ActionValidationConfig,
    depth: number,
    // oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- ctx is intentionally mutated in place as a running counter
    ctx: ActionValidationContext,
  ): unknown {
    if (isOpaqueActionArg(value)) return value

    if (depth > config.maxDepth)
      throw new TypeError(`Maximum nesting depth exceeded: ${depth} > ${config.maxDepth}`)

    if (value === null || typeof value === 'boolean') return value

    if (typeof value === 'string') {
      if (value.length > config.maxStringLength) {
        throw new TypeError(`String too long: ${value.length} > ${config.maxStringLength}`)
      }
      bumpActionValidationCount(ctx, value.length, config)
      return value
    }

    if (typeof value === 'number') {
      if (!Number.isFinite(value))
        throw new TypeError('Invalid number: Infinity or NaN not allowed')

      const absValue = Math.abs(value)
      if (absValue > 1e100) {
        const estimatedDigits = estimatedDigitCount(absValue)
        if (estimatedDigits > MAX_BIGINT_DIGITS) {
          throw new TypeError(
            `Number too large. Estimated ${estimatedDigits} digits but the limit is ${MAX_BIGINT_DIGITS}.`,
          )
        }
      }

      return value
    }

    if (Array.isArray(value)) {
      if (value.length > config.maxArrayLength) {
        throw new TypeError(`Array too large: ${value.length} > ${config.maxArrayLength}`)
      }
      if (value.length > 1) ctx.hasFork = true
      bumpActionValidationCount(ctx, value.length + 1, config)
      return value.map(item => validateActionValue(item, config, depth + 1, ctx))
    }

    if (typeof value === 'bigint') {
      const digits = value < 0n ? value.toString().slice(1) : value.toString()
      if (digits.length > MAX_BIGINT_DIGITS) {
        throw new TypeError(
          `BigInt too large: ${digits.length} digits but the limit is ${MAX_BIGINT_DIGITS}.`,
        )
      }
      bumpActionValidationCount(ctx, digits.length, config)
      return value
    }

    if (typeof Date !== 'undefined' && value instanceof Date) return value

    if (typeof Map !== 'undefined' && value instanceof Map)
      throw new TypeError('Map is not supported in server action arguments')

    if (typeof Set !== 'undefined' && value instanceof Set)
      throw new TypeError('Set is not supported in server action arguments')

    if (
      typeof ArrayBuffer !== 'undefined' &&
      (value instanceof ArrayBuffer || ArrayBuffer.isView(value))
    )
      return value

    if (typeof value === 'object') {
      const entries = Object.entries(value)
      if (entries.length > config.maxObjectKeys) {
        throw new TypeError(`Too many object keys: ${entries.length} > ${config.maxObjectKeys}`)
      }

      const sanitized: Record<string, unknown> = {}
      for (const [key, entryValue] of entries) {
        if (isDangerousActionProperty(key)) continue
        sanitized[key] = validateActionValue(entryValue, config, depth + 1, ctx)
      }

      return sanitized
    }

    return value
  }

  function validateActionArgsWithConfig(
    args: readonly unknown[],
    config: ActionValidationConfig,
  ): unknown[] {
    if (args.length > MAX_BOUND_ARGS) {
      throw new TypeError(
        `Server Function has too many bound arguments. Received ${args.length} but the limit is ${MAX_BOUND_ARGS}.`,
      )
    }

    const ctx: ActionValidationContext = { totalElements: 0, hasFork: false }
    return args.map(arg => validateActionValue(arg, config, 0, ctx))
  }

  function validateFormDataWithConfig(formData: FormData, config: ActionValidationConfig): void {
    let fieldCount = 0

    for (const [key, value] of formData.entries()) {
      fieldCount++
      if (fieldCount > MAX_FORM_FIELDS) {
        throw new TypeError(`Too many form fields: ${fieldCount} > ${MAX_FORM_FIELDS}`)
      }

      if (isFlightFormMetadataKey(key)) continue

      if (typeof value === 'string' && value.length > config.maxStringLength) {
        throw new TypeError(
          `Form field "${key}" too long: ${value.length} > ${config.maxStringLength}`,
        )
      }
    }
  }

  g.__RARI_ACTION_ARGS_VALIDATION__ = {
    productionValidationConfig,
    developmentValidationConfig,
    validateActionArgsWithConfig,
    validateFormDataWithConfig,
    isDangerousActionProperty,
  }
})(globalThis.g)
