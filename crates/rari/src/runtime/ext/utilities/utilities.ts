/// <reference path="../types.d.ts" />

if (typeof g === 'undefined') {
  Object.defineProperty(globalThis, 'g', {
    value: globalThis,
    writable: false,
    enumerable: false,
    configurable: false,
  })
}

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

export const nonEnumerable = (value: unknown): PropertyDescriptor => ObjectProperties.apply(value, 'nonEnumerable')
export const readOnly = (value: unknown): PropertyDescriptor => ObjectProperties.apply(value, 'readOnly')
export const writeable = (value: unknown): PropertyDescriptor => ObjectProperties.apply(value, 'writeable')

export function getterOnly(getter: () => unknown): PropertyDescriptor {
  return {
    get: getter,
    set() {},
    ...ObjectProperties.getterOnly,
  }
}

export const applyToGlobal = (properties: PropertyDescriptorMap) => Object.defineProperties(globalThis, properties)
export const applyToDeno = (properties: PropertyDescriptorMap) => Object.defineProperties(g.Deno, properties)
