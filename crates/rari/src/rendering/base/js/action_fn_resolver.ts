/// <reference path="../../types.d.ts" />

type ActionModule = Record<string, unknown> & { default?: unknown }

// eslint-disable-next-line unused-imports/no-unused-vars
function resolveActionFn(
  id: string,
  manifest: Record<string, { id: string, chunks: string[], name?: string }>,
): (...args: unknown[]) => unknown {
  const hashIdx = id.lastIndexOf('#')
  const moduleId = hashIdx === -1 ? id : id.slice(0, hashIdx)
  const exportName = hashIdx === -1 ? 'default' : id.slice(hashIdx + 1)

  const entry = manifest[id] ?? manifest[moduleId]
  const requireId = entry?.id ?? moduleId
  const requireFn = g.__rari_rsc_require__ as ((moduleId: string) => ActionModule | undefined) | undefined
  const mod = requireFn?.(requireId)

  if (!mod)
    throw new TypeError(`Server action module "${moduleId}" not found`)

  const fn = exportName === 'default'
    ? (mod.default ?? mod[exportName])
    : mod[exportName]

  if (typeof fn !== 'function')
    throw new TypeError(`Server action "${id}" is not a function`)

  return fn as (...args: unknown[]) => unknown
}
