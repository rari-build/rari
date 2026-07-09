/// <reference path="../../types.d.ts" />

type ActionModule = Record<string, unknown> & { default?: unknown }
type ActionFn = (...args: unknown[]) => unknown

function resolveActionModuleNamespace(
  id: string,
  manifest: Record<string, { id: string, chunks: string[], name?: string }>,
): ActionModule | undefined {
  const ssrModules = g['~rari']?.ssrModules
  if (!ssrModules)
    return undefined

  const hashIdx = id.lastIndexOf('#')
  const colonIdx = id.lastIndexOf(':')
  if (hashIdx === -1 && colonIdx === -1)
    return undefined

  const moduleId = hashIdx !== -1 ? id.slice(0, hashIdx) : id.slice(0, colonIdx)
  const entry = manifest[id] ?? manifest[moduleId]
  return ssrModules[id] ?? ssrModules[entry?.id ?? moduleId]
}

function resolveActionFnFromSsrModules(
  id: string,
  manifest: Record<string, { id: string, chunks: string[], name?: string }>,
): ActionFn | null {
  const hashIdx = id.lastIndexOf('#')
  const colonIdx = id.lastIndexOf(':')
  const exportName = hashIdx !== -1
    ? id.slice(hashIdx + 1)
    : colonIdx !== -1
      ? id.slice(colonIdx + 1)
      : 'default'

  const mod = resolveActionModuleNamespace(id, manifest)
  if (!mod)
    return null

  const moduleId = hashIdx !== -1
    ? id.slice(0, hashIdx)
    : colonIdx !== -1
      ? id.slice(0, colonIdx)
      : id
  const entry = manifest[id] ?? manifest[moduleId]
  const fnName = entry?.name ?? exportName
  const fn = fnName === 'default'
    ? (mod.default ?? mod[fnName])
    : mod[fnName]

  return typeof fn === 'function' ? fn as ActionFn : null
}

// eslint-disable-next-line unused-imports/no-unused-vars
function resolveActionFn(
  id: string,
  manifest: Record<string, { id: string, chunks: string[], name?: string }>,
): ActionFn {
  const fn = (typeof g.getServerFunction === 'function'
    ? g.getServerFunction(id)
    : null) ?? resolveActionFnFromSsrModules(id, manifest)

  if (typeof fn === 'function')
    return fn

  const hashIdx = id.lastIndexOf('#')
  const moduleId = hashIdx === -1 ? id : id.slice(0, hashIdx)
  if (!resolveActionModuleNamespace(id, manifest))
    throw new TypeError(`Server action module "${moduleId}" not found`)

  throw new TypeError(`Server action "${id}" is not a function`)
}
