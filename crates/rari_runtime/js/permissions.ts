// Copyright 2018-2026 the Deno authors. MIT license.
/* oxlint-disable typescript/prefer-readonly-parameter-types */

const { core, primordials } = __bootstrap
const { op_query_permission, op_request_permission, op_revoke_permission } = core.ops
const {
  ArrayIsArray,
  ArrayPrototypeMap,
  ArrayPrototypeSlice,
  MapPrototypeGet,
  MapPrototypeHas,
  MapPrototypeSet,
  PromiseResolve,
  PromiseReject,
  ReflectHas,
  SafeArrayIterator,
  SafeMap,
  Symbol,
  SymbolFor,
  TypeError,
} = primordials

const { pathFromURL } = core.loadExtScript('ext:deno_web/00_infra.js')
const { Event, EventTarget } = core.loadExtScript('ext:deno_web/02_event.js')

const illegalConstructorKey = Symbol('illegalConstructorKey')

type PermissionName = Deno.PermissionName
interface RawPermissionStatus {
  readonly state: Deno.PermissionState
  readonly partial: boolean
}

interface StatusCacheValue {
  state: Deno.PermissionState
  partial: boolean
  status?: PermissionStatus
}

const permissionNames: readonly PermissionName[] = [
  'read',
  'write',
  'net',
  'env',
  'sys',
  'run',
  'ffi',
  'import',
]

class PermissionStatus extends EventTarget {
  readonly #status: RawPermissionStatus

  onchange: ((this: PermissionStatus, event: Event) => unknown) | null = null

  get state(): Deno.PermissionState {
    return this.#status.state
  }

  get partial(): boolean {
    return this.#status.partial
  }

  constructor(status: RawPermissionStatus | null = null, key: unknown = null) {
    if (key !== illegalConstructorKey) {
      throw new TypeError('Illegal constructor')
    }
    super()
    if (status === null) {
      throw new TypeError('Illegal constructor')
    }
    this.#status = status
  }

  dispatchEvent(event: Event): boolean {
    let dispatched = super.dispatchEvent(event)
    if (dispatched && this.onchange) {
      this.onchange(event)
      dispatched = !event.defaultPrevented
    }
    return dispatched
  }

  [SymbolFor('Deno.privateCustomInspect')](
    inspect: (value: unknown, options: unknown) => string,
    inspectOptions: unknown,
  ) {
    const object: {
      state: Deno.PermissionState
      onchange: ((this: PermissionStatus, event: Event) => unknown) | null
      partial?: boolean
    } = { state: this.state, onchange: this.onchange }
    if (this.partial) object.partial = this.partial
    return `${this.constructor.name} ${inspect(object, inspectOptions)}`
  }
}

const statusCache = new SafeMap<string, StatusCacheValue>()

function permissionCacheKey(desc: Deno.PermissionDescriptor): string {
  let { name: key } = desc
  if (
    (desc.name === 'read' || desc.name === 'write' || desc.name === 'ffi') &&
    ReflectHas(desc, 'path')
  ) {
    key += `-${desc.path}&`
  } else if ((desc.name === 'net' || desc.name === 'import') && ReflectHas(desc, 'host')) {
    key += `-${desc.host}&`
  } else if (desc.name === 'run' && desc.command !== undefined) {
    key += `-${desc.command}&`
  } else if (desc.name === 'env' && desc.variable !== undefined) {
    key += `-${desc.variable}&`
  } else if (desc.name === 'sys' && desc.kind !== undefined) {
    key += `-${desc.kind}&`
  } else {
    key += '$'
  }
  return key
}

function cache(desc: Deno.PermissionDescriptor, rawStatus: RawPermissionStatus): PermissionStatus {
  const key = permissionCacheKey(desc)
  if (MapPrototypeHas(statusCache, key)) {
    const cachedObj = MapPrototypeGet(statusCache, key)
    if (cachedObj === undefined) {
      throw new TypeError('Permission status cache entry is missing')
    }
    if (cachedObj.state !== rawStatus.state || cachedObj.partial !== rawStatus.partial) {
      cachedObj.state = rawStatus.state
      cachedObj.partial = rawStatus.partial
      cachedObj.status?.dispatchEvent(new Event('change', { cancelable: false }))
    }
    if (cachedObj.status === undefined) {
      throw new TypeError('Permission status cache entry is missing')
    }
    return cachedObj.status
  }
  const obj: StatusCacheValue = rawStatus
  obj.status = new PermissionStatus(obj, illegalConstructorKey)
  MapPrototypeSet(statusCache, key, obj)
  return obj.status
}

function isValidDescriptor(desc: unknown): desc is Deno.PermissionDescriptor {
  if (typeof desc !== 'object' || desc === null || !('name' in desc)) {
    return false
  }
  const name: unknown = desc.name
  return typeof name === 'string' && permissionNames.some(permissionName => permissionName === name)
}

function formDescriptor(desc: Deno.PermissionDescriptor): Deno.PermissionDescriptor {
  if (desc.name === 'read' || desc.name === 'write' || desc.name === 'ffi') {
    if (desc.path !== undefined) {
      return { ...desc, name: desc.name, path: pathFromURL(desc.path) }
    }
  } else if (desc.name === 'run' && desc.command !== undefined) {
    return { ...desc, name: desc.name, command: pathFromURL(desc.command) }
  }
  return desc
}

class Permissions {
  constructor(key: unknown = null) {
    if (key !== illegalConstructorKey) {
      throw new TypeError('Illegal constructor')
    }
  }

  async query(desc: unknown): Promise<PermissionStatus> {
    try {
      return await PromiseResolve(this.querySync(desc))
    } catch (error) {
      return PromiseReject(error)
    }
  }

  querySync(desc: unknown): PermissionStatus {
    if (!isValidDescriptor(desc)) {
      throw new TypeError('The provided value is not a valid permission name')
    }

    const normalized = formDescriptor(desc)
    const status = op_query_permission(normalized)
    return cache(normalized, status)
  }

  async revoke(desc: unknown): Promise<PermissionStatus> {
    try {
      return await PromiseResolve(this.revokeSync(desc))
    } catch (error) {
      return PromiseReject(error)
    }
  }

  revokeSync(desc: unknown): PermissionStatus {
    if (!isValidDescriptor(desc)) {
      throw new TypeError('The provided value is not a valid permission name')
    }

    const normalized = formDescriptor(desc)
    const status = op_revoke_permission(normalized)
    return cache(normalized, status)
  }

  async request(desc: unknown): Promise<PermissionStatus> {
    try {
      return await PromiseResolve(this.requestSync(desc))
    } catch (error) {
      return PromiseReject(error)
    }
  }

  requestSync(desc: unknown): PermissionStatus {
    if (!isValidDescriptor(desc)) {
      throw new TypeError('The provided value is not a valid permission name.')
    }

    const normalized = formDescriptor(desc)
    const status = op_request_permission(normalized)
    return cache(normalized, status)
  }
}

const permissions = new Permissions(illegalConstructorKey)

function isPermissionRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function serializePermissions(permissions: unknown): unknown {
  if (isPermissionRecord(permissions)) {
    const permissionRecord = permissions
    const serializedPermissions: Record<string, unknown> = { __proto__: null }
    for (const key of new SafeArrayIterator(['read', 'write', 'run', 'ffi', 'import'])) {
      if (ArrayIsArray(permissionRecord[key])) {
        serializedPermissions[key] = ArrayPrototypeMap(permissionRecord[key], path => {
          if (typeof path !== 'string' && !(path instanceof URL)) {
            return path
          }
          return pathFromURL(path)
        })
      } else {
        serializedPermissions[key] = permissionRecord[key]
      }
    }
    for (const key of new SafeArrayIterator(['env', 'net', 'sys'])) {
      if (ArrayIsArray(permissionRecord[key])) {
        serializedPermissions[key] = ArrayPrototypeSlice(permissionRecord[key])
      } else {
        serializedPermissions[key] = permissionRecord[key]
      }
    }
    return serializedPermissions
  }
  return permissions
}

export { Permissions, permissions, PermissionStatus, serializePermissions }
