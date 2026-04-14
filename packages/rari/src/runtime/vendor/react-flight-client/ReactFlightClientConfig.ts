/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * Vendored from: https://github.com/facebook/react
 * Original file: packages/react-client/src/ReactFlightClientConfig.js (shim)
 * Reference impl: packages/react-server-dom-webpack/src/client/ReactFlightClientConfigBundlerWebpack.js
 * Modifications: Adapted for rari's client component registry
 */

export interface ClientReferenceMetadata {
  id: string
  chunks: Array<string>
  name: string
}

// eslint-disable-next-line unused-imports/no-unused-vars
export interface ClientReference<T> {
  specifier: string
  name: string
}

export type ServerConsumerModuleMap = Record<string, any>

export interface StringDecoder {
  decode: (chunk: Uint8Array, options?: { stream?: boolean }) => string
}

interface GlobalWithRari {
  '~clientComponents'?: Record<string, ComponentInfo>
}

interface ComponentInfo {
  component?: any
  loader?: () => Promise<any>
  loading?: boolean
  loadPromise?: Promise<any>
  registered?: boolean
  name?: string
}

function getGlobalThis(): GlobalWithRari {
  return globalThis as unknown as GlobalWithRari
}

export function resolveClientReference<T>(
  metadata: ClientReferenceMetadata,
): ClientReference<T> {
  return {
    specifier: metadata.id,
    name: metadata.name || 'default',
  }
}

export function preloadModule<T>(
  moduleReference: ClientReference<T>,
): null | Promise<any> {
  const clientComponents = getGlobalThis()['~clientComponents']
  if (!clientComponents) {
    return null
  }

  const key = `${moduleReference.specifier}#${moduleReference.name}`
  const componentInfo = clientComponents[key] || clientComponents[moduleReference.specifier]

  if (!componentInfo) {
    return null
  }

  if (componentInfo.component)
    return null

  if (componentInfo.loadPromise)
    return componentInfo.loadPromise

  if (componentInfo.loader) {
    componentInfo.loading = true
    componentInfo.loadPromise = componentInfo.loader()
      .then((module: any) => {
        componentInfo.component = module.default || module
        componentInfo.registered = true
        componentInfo.loading = false
        return module
      })
      .catch((error: any) => {
        componentInfo.loading = false
        componentInfo.loadPromise = undefined
        throw error
      })
    return componentInfo.loadPromise
  }

  return null
}

export function requireModule<T>(moduleReference: ClientReference<T>): T {
  const clientComponents = getGlobalThis()['~clientComponents']
  if (!clientComponents) {
    throw new Error('[rari] Client components registry not found')
  }

  const key = `${moduleReference.specifier}#${moduleReference.name}`
  const componentInfo = clientComponents[key] || clientComponents[moduleReference.specifier]

  if (!componentInfo) {
    throw new Error(`[rari] Component not registered: ${key}`)
  }

  if (componentInfo.component) {
    const component = componentInfo.component
    if (typeof component === 'function')
      return component

    const result = moduleReference.name === 'default' ? component : component?.[moduleReference.name]
    return result
  }

  if (componentInfo.loadPromise)
    throw componentInfo.loadPromise

  if (componentInfo.loader) {
    componentInfo.loading = true
    componentInfo.loadPromise = componentInfo.loader()
      .then((module: any) => {
        componentInfo.component = module.default || module
        componentInfo.registered = true
        componentInfo.loading = false
        return module
      })
      .catch((error: any) => {
        componentInfo.loading = false
        componentInfo.loadPromise = undefined
        throw error
      })
    throw componentInfo.loadPromise
  }

  throw new Error(`[rari] Module not found: ${key}`)
}

export function readPartialStringChunk(
  decoder: StringDecoder,
  buffer: Uint8Array,
): string {
  return decoder.decode(buffer, { stream: true })
}

export function readFinalStringChunk(
  decoder: StringDecoder,
  buffer: Uint8Array,
): string {
  return decoder.decode(buffer)
}

export function createStringDecoder(): StringDecoder {
  return new TextDecoder()
}
