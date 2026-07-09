/// <reference path="../../types.d.ts" />

type ActionModule = Record<string, unknown> & { default?: unknown }
type ActionFn = (...args: unknown[]) => unknown

interface ParsedActionId {
  moduleId: string
  exportName: string
}

function parseActionId(id: string): ParsedActionId | null {
  const hashIdx = id.lastIndexOf('#')
  const colonIdx = id.lastIndexOf(':')
  if (hashIdx === -1 && colonIdx === -1)
    return null

  const separatorIdx = hashIdx !== -1 ? hashIdx : colonIdx
  return {
    moduleId: id.slice(0, separatorIdx),
    exportName: id.slice(separatorIdx + 1),
  }
}

function resolveActionModuleNamespace(
  id: string,
  manifest: Record<string, { id: string, chunks: string[], name?: string }>,
): ActionModule | undefined {
  const ssrModules = g['~rari']?.ssrModules
  if (!ssrModules)
    return undefined

  const parsed = parseActionId(id)
  if (!parsed)
    return undefined

  const entry = manifest[id] ?? manifest[parsed.moduleId]
  return ssrModules[id] ?? ssrModules[entry?.id ?? parsed.moduleId]
}

function resolveActionFnFromSsrModules(
  id: string,
  manifest: Record<string, { id: string, chunks: string[], name?: string }>,
): ActionFn | null {
  const parsed = parseActionId(id)
  const exportName = parsed?.exportName ?? 'default'

  const mod = resolveActionModuleNamespace(id, manifest)
  if (!mod)
    return null

  const moduleId = parsed?.moduleId ?? id
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

  const parsed = parseActionId(id)
  const moduleId = parsed?.moduleId ?? id
  if (!resolveActionModuleNamespace(id, manifest))
    throw new TypeError(`Server action module "${moduleId}" not found`)

  throw new TypeError(`Server action "${id}" is not a function`)
}
