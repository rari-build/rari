import { registerClientReference } from '@/runtime/rsc/references'
import { getClientComponents } from '@/runtime/shared/rari-global'
import { EXTENSION_REGEX } from '@/shared/regex-constants'

interface MDXClientComponentConfig {
  readonly component: unknown
  readonly id: string
  readonly exportName?: string
}

export function createMDXClientReference(
  component: unknown,
  id: string,
  exportName: string = 'default',
) {
  const key = `${id}#${exportName}`

  if (typeof globalThis !== 'undefined') {
    const clientComponents = getClientComponents()
    const pathSegment = id.replace(EXTENSION_REGEX, '').split('/').pop()
    const componentId = pathSegment != null && pathSegment !== '' ? pathSegment : exportName

    const componentEntry = {
      id: exportName === 'default' ? componentId : exportName,
      path: id,
      type: 'client',
      component,
      registered: true,
    }

    clientComponents[key] = componentEntry
    clientComponents[componentId] = componentEntry
    clientComponents[id] = componentEntry
  }

  function clientProxy(): never {
    throw new Error(
      `Attempted to call ${exportName}() from the server but ${exportName} is on the client. ` +
        `It's not possible to invoke a client function from the server, it can only be rendered as a Component or passed to props of a Client Component.`,
    )
  }

  return registerClientReference(clientProxy, id, exportName)
}

export function createMDXClientReferences(
  components: Readonly<{ readonly [key: string]: MDXClientComponentConfig }>,
): Record<string, any> {
  const references: Record<string, any> = {}

  for (const [name, { component, id, exportName = 'default' }] of Object.entries(components))
    references[name] = createMDXClientReference(component, id, exportName)

  return references
}
