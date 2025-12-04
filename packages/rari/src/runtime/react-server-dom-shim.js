const clientReferenceRegistry = new Map()
const serverReferenceRegistry = new Map()

let rustBridge = null

if (typeof globalThis.__rari_bridge !== 'undefined') {
  rustBridge = globalThis.__rari_bridge
}

export function registerClientReference(clientReference, id, exportName) {
  const key = `${id}#${exportName}`
  clientReferenceRegistry.set(key, {
    id,
    exportName,
    chunks: [id],
    name: exportName,
    async: false,
  })

  Object.defineProperty(clientReference, '$typeof', {
    value: Symbol.for('react.client.reference'),
    enumerable: false,
  })

  Object.defineProperty(clientReference, '$id', {
    value: key,
    enumerable: false,
  })

  Object.defineProperty(clientReference, '$async', {
    value: false,
    enumerable: false,
  })

  try {
    if (rustBridge && typeof rustBridge.registerClientReference === 'function') {
      rustBridge.registerClientReference(key, id, exportName)
    }
  }
  catch (error) {
    console.error(error)
  }

  return clientReference
}

const clientComponentRegistry = new Map()

export function registerClientComponent(componentFunction, id, exportName) {
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

export function getClientComponent(id) {
  return clientComponentRegistry.get(id)
}

export function registerServerReference(serverReference, id, exportName) {
  const key = `${id}#${exportName}`
  serverReferenceRegistry.set(key, {
    id,
    exportName,
    bound: false,
  })

  Object.defineProperty(serverReference, '$typeof', {
    value: Symbol.for('react.server.reference'),
    enumerable: false,
  })

  Object.defineProperty(serverReference, '$id', {
    value: key,
    enumerable: false,
  })

  Object.defineProperty(serverReference, '$bound', {
    value: false,
    enumerable: false,
  })

  try {
    if (rustBridge && typeof rustBridge.registerServerReference === 'function') {
      rustBridge.registerServerReference(key, id, exportName)
    }
  }
  catch (error) {
    console.error(error)
  }

  return serverReference
}

export function createClientModuleProxy(id) {
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

export function createClientModuleMap() {
  const moduleMap = {}

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
