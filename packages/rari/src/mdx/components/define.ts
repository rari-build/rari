import type { ComponentType } from 'react'
import { isRecord } from '@/shared/utils/type-guards'
import { scanMdxComponentNames } from '../scan/names'
import { createMDXClientReferences } from './client-refs'

export interface MdxComponentEntry {
  readonly name: string
  readonly component: ComponentType<any>
  readonly id: string
  readonly client?: boolean
  readonly exportName?: string
}

type MdxComponentsInput = Readonly<Record<string, ComponentType<any> | MdxComponentEntry>>

function isResolvedEntry(value: unknown): value is MdxComponentEntry {
  return isRecord(value) && typeof value.id === 'string' && 'component' in value
}

/* oxlint-disable typescript/prefer-readonly-parameter-types -- union with react's ComponentType loses its allow-listed alias and expands to mutable ComponentClass/FunctionComponent members */
function isEntryArray(
  input: readonly MdxComponentEntry[] | MdxComponentsInput,
): input is readonly MdxComponentEntry[] {
  return Array.isArray(input)
}

function normalizeRegistry(
  input: readonly MdxComponentEntry[] | MdxComponentsInput,
): MdxComponentEntry[] {
  if (isEntryArray(input)) return [...input]

  return Object.entries(input).map(([name, value]) => {
    if (isResolvedEntry(value)) {
      return {
        component: value.component,
        id: value.id,
        exportName: value.exportName,
        name,
        client: value.client ?? true,
      }
    }

    throw new Error(
      `[rari/mdx] Component "${name}" is missing module metadata. ` +
        'Pass components to defineMdxComponents({ ... }) in a file processed by the rari vite plugin.',
    )
  })
}

/* oxlint-enable typescript/prefer-readonly-parameter-types */

export function defineMdxComponents(
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- union with react's ComponentType loses its allow-listed alias and expands to mutable ComponentClass/FunctionComponent members
  input: readonly MdxComponentEntry[] | MdxComponentsInput,
): (content: string) => Record<string, any> {
  const registry = normalizeRegistry(input)

  return (content: string) => {
    const result: Record<string, any> = {}
    const clientComponents: Record<string, { component: any; id: string; exportName?: string }> = {}
    const usedComponentNames = new Set(scanMdxComponentNames(content))

    for (const entry of registry) {
      if (!usedComponentNames.has(entry.name)) continue

      if (entry.client === false) {
        result[entry.name] = entry.component
        continue
      }

      clientComponents[entry.name] = {
        component: entry.component,
        id: entry.id,
        exportName: entry.exportName,
      }
    }

    return {
      ...result,
      ...createMDXClientReferences(clientComponents),
    }
  }
}
