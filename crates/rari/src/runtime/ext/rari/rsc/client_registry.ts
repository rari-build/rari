/// <reference path="../core/types.d.ts" />

interface ComponentInfo {
  readonly id: string
  readonly path: string
  readonly type: 'client'
  readonly component: unknown
  readonly registered: boolean
}

/** Extra metadata that may be attached to a function/component value at runtime. */
interface ClientComponentCandidate {
  'name'?: string
  'displayName'?: string
  '~isClientComponent'?: boolean
  '~clientComponentId'?: string
  '$$typeof'?: symbol
  '$$id'?: string
  '$$async'?: boolean
}

interface ClientReference {
  '$$typeof': symbol
  '$$id': string
  '$$async': boolean
  'name': string
  '~isClientComponent': boolean
}

if (typeof g['~clientComponents'] === 'undefined') g['~clientComponents'] = {}
if (typeof g['~clientComponentNames'] === 'undefined') g['~clientComponentNames'] = {}
if (typeof g['~clientComponentPaths'] === 'undefined') g['~clientComponentPaths'] = {}

const REACT_CLIENT_REFERENCE = Symbol.for('react.client.reference')

function getComponentDisplayName(componentType: unknown): string | undefined {
  if (typeof componentType !== 'function') return undefined

  if (componentType.name !== '') return componentType.name

  if ('displayName' in componentType) {
    const displayName = (componentType as { displayName?: unknown }).displayName
    if (typeof displayName === 'string' && displayName !== '') return displayName
  }

  return undefined
}

function hasClientComponentFlag(componentType: unknown): boolean {
  if (typeof componentType !== 'function') return false

  return Reflect.get(componentType, '~isClientComponent') === true
}

function isClientReferenceObject(componentType: object): boolean {
  return Reflect.get(componentType, '$$typeof') === REACT_CLIENT_REFERENCE
}

function registerClientComponent(
  componentId: string,
  componentPath: string,
  component?: unknown,
): void {
  if (componentId === '' || componentPath === '') {
    console.warn('registerClientComponent: componentId and componentPath are required')
    return
  }

  const componentInfo: ComponentInfo = {
    id: componentId,
    path: componentPath,
    type: 'client',
    component: component ?? null,
    registered: true,
  }

  g['~clientComponents']![componentId] = componentInfo
  g['~clientComponentPaths']![componentPath] = componentId

  if (component != null && typeof component === 'object') {
    const candidate = component as ClientComponentCandidate
    if (
      (candidate.name != null && candidate.name !== '') ||
      (candidate.displayName != null && candidate.displayName !== '')
    ) {
      const componentName =
        candidate.name != null && candidate.name !== '' ? candidate.name : candidate.displayName
      if (componentName != null && componentName !== '')
        g['~clientComponentNames']![componentName] = componentId
    }
  }

  const pathName = extractComponentNameFromPath(componentPath)

  if (pathName != null && pathName !== '') g['~clientComponentNames']![pathName] = componentId
}

function isClientComponent(
  componentType: unknown,
  registry?: Readonly<{ readonly [key: string]: ComponentInfo }>,
): boolean {
  const clientRegistry = registry ?? g['~clientComponents'] ?? {}

  if (typeof componentType === 'function') {
    const componentName = getComponentDisplayName(componentType)

    if (
      componentName != null &&
      componentName !== '' &&
      g['~clientComponentNames']?.[componentName] != null
    )
      return true
    if (hasClientComponentFlag(componentType)) return true
  }

  if (
    typeof componentType === 'object' &&
    componentType !== null &&
    isClientReferenceObject(componentType)
  )
    return true

  if (typeof componentType === 'string' && componentType in clientRegistry) return true

  return false
}

function getClientComponentInfo(componentType: unknown): ComponentInfo | null {
  if (componentType == null) return null

  if (typeof componentType === 'function') {
    const componentName = getComponentDisplayName(componentType)

    if (
      componentName != null &&
      componentName !== '' &&
      g['~clientComponentNames']?.[componentName] != null
    ) {
      const componentId = g['~clientComponentNames'][componentName]
      return g['~clientComponents']?.[componentId] ?? null
    }
  }

  if (typeof componentType === 'string' && g['~clientComponents']?.[componentType])
    return g['~clientComponents'][componentType]

  if (typeof componentType === 'object' && isClientReferenceObject(componentType)) {
    const rawId: unknown = Reflect.get(componentType, '$$id')
    const rawName: unknown = Reflect.get(componentType, 'name')
    const componentId: unknown = rawId ?? rawName ?? 'UnknownClient'
    const isAsync = Reflect.get(componentType, '$$async') === true
    return {
      id: typeof componentId === 'string' ? componentId : 'UnknownClient',
      path: isAsync ? 'async' : 'unknown',
      type: 'client',
      component: componentType,
      registered: false,
    }
  }

  return null
}

function getClientComponentId(componentType: unknown): string | null {
  const info = getClientComponentInfo(componentType)
  return info ? info.id : null
}

const PATH_SEPARATOR_REGEX = /[/\\]/
const FILE_EXTENSION_REGEX = /\.(?:js|jsx|ts|tsx)$/

function extractComponentNameFromPath(componentPath: string): string | null {
  if (!componentPath || typeof componentPath !== 'string') return null

  const pathParts = componentPath.split(PATH_SEPARATOR_REGEX)
  const fileName = pathParts.at(-1)

  if (fileName == null || fileName === '') return null

  const nameWithoutExt = fileName.replace(FILE_EXTENSION_REGEX, '')

  if (nameWithoutExt.toLowerCase() === 'index') {
    const parentDir = pathParts.at(-2)
    return parentDir != null && parentDir !== '' ? parentDir : null
  }

  return nameWithoutExt
}

function listClientComponents(): Record<string, ComponentInfo> {
  return { ...(g['~clientComponents'] ?? {}) }
}

function listClientComponentNames(): Record<string, string> {
  return { ...(g['~clientComponentNames'] ?? {}) }
}

function clearClientComponents(): void {
  g['~clientComponents'] = {}
  g['~clientComponentNames'] = {}
  g['~clientComponentPaths'] = {}
}

function registerClientComponentFromModule(componentPath: string, moduleExports: unknown): void {
  if (componentPath === '' || moduleExports == null || typeof moduleExports !== 'object') return

  if ('default' in moduleExports && typeof moduleExports.default === 'function') {
    const componentName = extractComponentNameFromPath(componentPath)
    const componentId =
      componentName != null && componentName !== '' ? componentName : 'DefaultExport'
    registerClientComponent(componentId, componentPath, moduleExports.default)
  }

  for (const [exportName, exportValue] of Object.entries(moduleExports)) {
    if (typeof exportValue === 'function' && exportName !== 'default')
      registerClientComponent(exportName, componentPath, exportValue)
  }
}

function markAsClientComponent(component: unknown, componentId?: string): void {
  if (typeof component !== 'function') return

  Object.assign(component, { '~isClientComponent': true })

  if (componentId != null && componentId !== '')
    Object.assign(component, { '~clientComponentId': componentId })
}

function createClientReference(componentId: string, componentPath: string): ClientReference {
  const reference: ClientReference = {
    '$$typeof': REACT_CLIENT_REFERENCE,
    '$$id': componentId,
    '$$async': false,
    'name': componentId,
    '~isClientComponent': true,
  }

  registerClientComponent(componentId, componentPath, reference)

  return reference
}

if (typeof g !== 'undefined') {
  g.registerClientComponent = registerClientComponent
  g.isClientComponent = isClientComponent
  g.getClientComponentInfo = getClientComponentInfo
  g.getClientComponentId = getClientComponentId
  g.listClientComponents = listClientComponents
  g.listClientComponentNames = listClientComponentNames
  g.clearClientComponents = clearClientComponents
  g.registerClientComponentFromModule = registerClientComponentFromModule
  g.markAsClientComponent = markAsClientComponent
  g.createClientReference = createClientReference
}
