import {
  createClientModuleProxy as createClientModuleProxyBase,
  registerClientReference as registerClientReferenceBase,
  registerServerReference,
} from 'react-server-dom-webpack/server'
import '@/runtime/shared/types'

export { registerServerReference }

export const createClientModuleProxy = createClientModuleProxyBase

export function registerClientReference<T>(clientReference: T, id: string, exportName: string): T {
  const key = `${id}#${exportName}`
  const reference = registerClientReferenceBase(clientReference ?? (() => {}), id, exportName)

  try {
    const registerClientComponent: unknown = Reflect.get(globalThis, 'registerClientComponent')
    if (typeof registerClientComponent === 'function') {
      // oxlint-disable-next-line typescript/no-unsafe-call -- global hook registered at runtime
      Reflect.apply(registerClientComponent, undefined, [key, id, reference])
    }
  } catch (error) {
    console.error('[rari] Failed to register client reference:', error)
  }

  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- registerClientReferenceBase widens the client reference type
  return reference as T
}
