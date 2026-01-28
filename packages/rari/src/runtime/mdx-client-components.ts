import { registerClientComponent, registerClientReference } from './react-server-dom-shim.js'

export interface MDXClientComponentConfig {
  component: any
  id: string
  exportName?: string
}

export function createMDXClientReference(
  component: any,
  id: string,
  exportName: string = 'default',
): any {
  const key = `${id}#${exportName}`

  registerClientComponent(component, id, exportName)

  if (typeof globalThis !== 'undefined') {
    const globalAny = globalThis as any
    if (!globalAny['~clientComponents'])
      globalAny['~clientComponents'] = {}

    const componentId = id.replace(/\.[^.]+$/, '').split('/').pop() || exportName

    const componentEntry = {
      id: exportName === 'default' ? componentId : exportName,
      path: id,
      type: 'client',
      component,
      registered: true,
    }

    globalAny['~clientComponents'][key] = componentEntry
    globalAny['~clientComponents'][componentId] = componentEntry
    globalAny['~clientComponents'][id] = componentEntry
  }

  function clientProxy(): never {
    throw new Error(
      `Attempted to call ${exportName}() from the server but ${exportName} is on the client. `
      + `It's not possible to invoke a client function from the server, it can only be rendered as a Component or passed to props of a Client Component.`,
    )
  }

  return registerClientReference(clientProxy, id, exportName)
}

export function createMDXClientReferences(
  components: Record<string, MDXClientComponentConfig>,
): Record<string, any> {
  const references: Record<string, any> = {}

  for (const [name, { component, id, exportName = 'default' }] of Object.entries(components))
    references[name] = createMDXClientReference(component, id, exportName)

  return references
}
