import {
  registerClientReference as registerClientReferenceBase,
  registerServerReference,
} from 'react-server-dom-webpack/server'

export { registerServerReference }

export function registerClientReference<T>(
  clientReference: T,
  id: string,
  exportName: string,
): T {
  const key = `${id}#${exportName}`
  const reference = registerClientReferenceBase(
    clientReference ?? (() => {}),
    id,
    exportName,
  )

  try {
    const registerClientComponent = (globalThis as any).registerClientComponent
    if (typeof registerClientComponent === 'function')
      registerClientComponent(key, id, reference)
  }
  catch (error) {
    console.error('[rari] Failed to register client reference:', error)
  }

  return reference as T
}
