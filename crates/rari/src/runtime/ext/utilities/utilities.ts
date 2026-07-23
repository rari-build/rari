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
  nonEnumerable: {
    writable: true,
    enumerable: false,
    configurable: true,
  } as PropertyDescriptorConfig,
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

export function nonEnumerable(value: unknown): PropertyDescriptor {
  return ObjectProperties.apply(value, 'nonEnumerable')
}
export function readOnly(value: unknown): PropertyDescriptor {
  return ObjectProperties.apply(value, 'readOnly')
}
export function writeable(value: unknown): PropertyDescriptor {
  return ObjectProperties.apply(value, 'writeable')
}

export function getterOnly(getter: () => unknown): PropertyDescriptor {
  return {
    get: getter,
    set() {},
    ...ObjectProperties.getterOnly,
  }
}

export function applyToGlobal(properties: PropertyDescriptorMap) {
  return Object.defineProperties(globalThis, properties)
}
export function applyToDeno(properties: PropertyDescriptorMap) {
  return Object.defineProperties(g.Deno, properties)
}

const extScriptCache = new Map<string, unknown>()

// Loader factories intentionally parameterize only the return type.
/* oxlint-disable typescript/no-unnecessary-type-parameters -- Deno ext loaders */
export function loadExtScriptOnce<T>(specifier: string): T {
  let cached = extScriptCache.get(specifier)
  if (cached === undefined) {
    cached = core.loadExtScript(specifier)
    extScriptCache.set(specifier, cached)
  }

  return cached as T // oxlint-disable-line typescript/no-unsafe-type-assertion -- Deno ext scripts are dynamically loaded
}

export function lazyExtScript<T>(specifier: string): () => T {
  let mod: T | undefined
  return () => {
    mod ??= loadExtScriptOnce<T>(specifier)
    return mod
  }
}

const extModuleLoaderCache = new Map<string, () => unknown>()

export function lazyExtModule<T>(specifier: string): () => T {
  let loader = extModuleLoaderCache.get(specifier)
  if (loader == null) {
    loader = core.createLazyLoader<T>(specifier)
    extModuleLoaderCache.set(specifier, loader)
  }

  return loader as () => T // oxlint-disable-line typescript/no-unsafe-type-assertion -- lazy loader cache stores dynamic ext modules
}
/* oxlint-enable typescript/no-unnecessary-type-parameters */

export function nonEnumerableGetter(get: () => unknown): PropertyDescriptor {
  return {
    get,
    enumerable: false,
    configurable: true,
  }
}

/* oxlint-disable typescript/no-unnecessary-type-parameters -- select return type is only expressed via V */
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
    value(...args: readonly unknown[]) {
      const fn = select(load())
      if (typeof fn !== 'function')
        throw new TypeError('Expected lazy-loaded ext export to be a function')

      return Reflect.apply(fn, undefined, args) as unknown
    },
    writable: true,
    enumerable: true,
    configurable: true,
  }
}
/* oxlint-enable typescript/no-unnecessary-type-parameters */

export function defineDenoLazyProps<T>(load: () => T, keys: ReadonlyArray<keyof T & string>): void {
  const descriptors: PropertyDescriptorMap = {}

  for (const key of keys) {
    descriptors[key] = propNonEnumerableLazyLoaded(m => m[key], load)
  }

  Object.defineProperties(g.Deno, descriptors)
}
