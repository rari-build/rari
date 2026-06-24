/* oxlint-disable no-undef */
/* eslint-disable ts/ban-ts-comment */
/// <reference path="./types.d.ts" />

// @ts-ignore - Deno extension modules resolved at runtime
import 'ext:rari/cookies.ts'
// @ts-ignore - Deno extension modules resolved at runtime
import 'ext:rari/api_handler.ts'
// @ts-ignore - Deno extension modules resolved at runtime
import 'ext:rari/component_loader.ts'
// @ts-ignore - Deno extension modules resolved at runtime
import 'ext:rari/metadata_collector.ts'

type PropertyDescriptorType = 'nonEnumerable' | 'readOnly' | 'writeable' | 'getterOnly'

interface PropertyDescriptorConfig {
  writable?: boolean
  enumerable: boolean
  configurable: boolean
}

const ObjectProperties = {
  nonEnumerable: { writable: true, enumerable: false, configurable: true } as PropertyDescriptorConfig,
  readOnly: { writable: false, enumerable: false, configurable: true } as PropertyDescriptorConfig,
  writeable: { writable: true, enumerable: true, configurable: true } as PropertyDescriptorConfig,
  getterOnly: { enumerable: true, configurable: true } as PropertyDescriptorConfig,

  apply: (value: unknown, type: PropertyDescriptorType): PropertyDescriptor => {
    return {
      value,
      ...ObjectProperties[type],
    }
  },
}

const nonEnumerable = (value: unknown): PropertyDescriptor => ObjectProperties.apply(value, 'nonEnumerable')
const readOnly = (value: unknown): PropertyDescriptor => ObjectProperties.apply(value, 'readOnly')
const writeable = (value: unknown): PropertyDescriptor => ObjectProperties.apply(value, 'writeable')

function getterOnly(getter: () => unknown): PropertyDescriptor {
  return {
    get: getter,
    set() { },
    ...ObjectProperties.getterOnly,
  }
}

const applyToGlobal = (properties: PropertyDescriptorMap) => Object.defineProperties(globalThis, properties)
const applyToDeno = (properties: PropertyDescriptorMap) => Object.defineProperties(globalThis.Deno, properties)

export {
  applyToDeno,
  applyToGlobal,
  getterOnly,
  nonEnumerable,
  readOnly,
  writeable,
}
