interface ClientReferenceData {
  id: string
  exportName: string
  chunks: string[]
  name: string
  async: boolean
}

interface ServerReferenceData {
  id: string
  exportName: string
  bound: boolean
}

interface RustBridge {
  registerClientReference?: (key: string, id: string, exportName: string) => void
  registerServerReference?: (key: string, id: string, exportName: string) => void
}

interface GlobalWithRari {
  '~rari'?: {
    bridge?: RustBridge
  }
}

const clientReferenceRegistry = new Map<string, ClientReferenceData>()
const serverReferenceRegistry = new Map<string, ServerReferenceData>()

let rustBridge: RustBridge | null = null

if (typeof (globalThis as GlobalWithRari)['~rari']?.bridge !== 'undefined')
  rustBridge = (globalThis as GlobalWithRari)['~rari']!.bridge!

export function registerClientReference(clientReference: any, id: string, exportName: string): any {
  const key = `${id}#${exportName}`
  clientReferenceRegistry.set(key, {
    id,
    exportName,
    chunks: [id],
    name: exportName,
    async: false,
  })

  Object.defineProperty(clientReference, '$$typeof', {
    value: Symbol.for('react.client.reference'),
    enumerable: false,
  })

  Object.defineProperty(clientReference, '$$id', {
    value: key,
    enumerable: false,
  })

  Object.defineProperty(clientReference, '$$async', {
    value: false,
    enumerable: false,
  })

  try {
    if (rustBridge && typeof rustBridge.registerClientReference === 'function')
      rustBridge.registerClientReference(key, id, exportName)
  }
  catch (error) {
    console.error(error)
  }

  return clientReference
}

const clientComponentRegistry = new Map<string, any>()

export function registerClientComponent(componentFunction: any, id: string, exportName: string): void {
  const key = `${id}#${exportName}`
  clientComponentRegistry.set(key, componentFunction)
  clientReferenceRegistry.set(key, {
    id,
    exportName,
    chunks: [id],
    name: exportName,
    async: false,
  })
}

export function getClientComponent(id: string): any {
  return clientComponentRegistry.get(id)
}

export function registerServerReference(serverReference: any, id: string, exportName: string): any {
  const key = `${id}#${exportName}`
  serverReferenceRegistry.set(key, {
    id,
    exportName,
    bound: false,
  })

  Object.defineProperty(serverReference, '$$typeof', {
    value: Symbol.for('react.server.reference'),
    enumerable: false,
  })

  Object.defineProperty(serverReference, '$$id', {
    value: key,
    enumerable: false,
  })

  Object.defineProperty(serverReference, '$$bound', {
    value: false,
    enumerable: false,
  })

  try {
    if (rustBridge && typeof rustBridge.registerServerReference === 'function')
      rustBridge.registerServerReference(key, id, exportName)
  }
  catch (error) {
    console.error(error)
  }

  return serverReference
}

export function createClientModuleProxy(id: string): any {
  return new Proxy({}, {
    get(_target, prop) {
      function clientProxy() {
        throw new Error(
          `Attempted to call ${String(prop)}() from the server but ${String(prop)} is on the client. `
          + `It's not possible to invoke a client function from the server, it can only be rendered as a Component or passed to props of a Client Component.`,
        )
      }

      return registerClientReference(clientProxy, id, String(prop))
    },
  })
}

export const __CLIENT_REFERENCE_REGISTRY__ = clientReferenceRegistry
export const __SERVER_REFERENCE_REGISTRY__ = serverReferenceRegistry
export const __CLIENT_COMPONENT_REGISTRY__ = clientComponentRegistry

export function createClientModuleMap(): Record<string, any> {
  const moduleMap: Record<string, any> = {}

  for (const [key, componentData] of clientReferenceRegistry) {
    const component = clientComponentRegistry.get(key)
    if (component) {
      moduleMap[key] = {
        id: componentData.id,
        chunks: componentData.chunks,
        name: componentData.name,
        async: componentData.async,
        default: component,
      }
    }
  }

  return moduleMap
}
