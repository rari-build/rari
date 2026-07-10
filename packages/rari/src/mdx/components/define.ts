import type { ComponentType } from 'react'
import { scanMdxComponentNames } from '../scan/names'
import { createMDXClientReferences } from './client-refs'

export interface MdxComponentEntry {
  name: string
  component: ComponentType<any>
  id: string
  client?: boolean
  exportName?: string
}

type MdxComponentsInput = Record<string, ComponentType<any> | MdxComponentEntry>

function isResolvedEntry(value: ComponentType<any> | MdxComponentEntry): value is MdxComponentEntry {
  return typeof value === 'object' && value !== null && 'component' in value && 'id' in value
}

function normalizeRegistry(input: MdxComponentEntry[] | MdxComponentsInput): MdxComponentEntry[] {
  if (Array.isArray(input))
    return input

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
      `[rari/mdx] Component "${name}" is missing module metadata. `
      + 'Pass components to defineMdxComponents({ ... }) in a file processed by the rari vite plugin.',
    )
  })
}

export function defineMdxComponents(
  input: MdxComponentEntry[] | MdxComponentsInput,
): (content: string) => Record<string, any> {
  const registry = normalizeRegistry(input)

  return (content: string) => {
    const result: Record<string, any> = {}
    const clientComponents: Record<string, { component: any, id: string, exportName?: string }> = {}
    const usedComponentNames = new Set(scanMdxComponentNames(content))

    for (const entry of registry) {
      if (!usedComponentNames.has(entry.name))
        continue

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
