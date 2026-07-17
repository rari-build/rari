/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'

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

const extScriptCache = new Map<string, unknown>()

export function loadExtScriptOnce(specifier: string): unknown {
  let cached = extScriptCache.get(specifier)
  if (cached === undefined) {
    cached = core.loadExtScript(specifier)
    extScriptCache.set(specifier, cached)
  }

  return cached
}

export function lazyExtScript<T>(
  specifier: string,
): () => T {
  let mod: T | undefined
  return () => {
    if (!mod)
      mod = loadExtScriptOnce(specifier) as T

    return mod
  }
}

const extModuleLoaderCache = new Map<string, () => unknown>()

export function lazyExtModule<T>(specifier: string): () => T {
  let loader = extModuleLoaderCache.get(specifier) as (() => T) | undefined
  if (!loader) {
    loader = core.createLazyLoader<T>(specifier)
    extModuleLoaderCache.set(specifier, loader as () => unknown)
  }

  return loader
}

export function nonEnumerableGetter(get: () => unknown): PropertyDescriptor {
  return {
    get,
    enumerable: false,
    configurable: true,
  }
}

export function propNonEnumerableLazyLoaded<T, V>(
  select: (mod: T) => V,
  load: () => T,
): PropertyDescriptor {
  return nonEnumerableGetter((): V => select(load()))
}

export function propWritableLazyLoaded<T, V>(
  select: (mod: T) => V,
  load: () => T,
): PropertyDescriptor {
  return {
    value(...args: unknown[]) {
      const fn = select(load()) as (...args: unknown[]) => unknown
      return fn(...args)
    },
    writable: true,
    enumerable: true,
    configurable: true,
  }
}

export function defineDenoLazyProps<T>(
  load: () => T,
  keys: (keyof T & string)[],
): void {
  const descriptors: PropertyDescriptorMap = {}

  for (const key of keys) {
    descriptors[key] = propNonEnumerableLazyLoaded(m => m[key], load)
  }

  Object.defineProperties(g.Deno, descriptors)
}
