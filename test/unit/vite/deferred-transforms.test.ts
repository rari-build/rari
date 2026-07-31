import {
  buildClientReferenceStubModule,
  collectExportNames,
} from '@rari/vite/transform/client-reference-stub'
import {
  buildGlobalClientComponentWrapper,
  buildGlobalClientNamespaceWrapper,
} from '@rari/vite/transform/component-global'
import { transformInlineServerActions } from '@rari/vite/transform/inline-server-action'
import { describe, expect, it } from 'vite-plus/test'

describe('collectExportNames', () => {
  it('collects default and named declarations', () => {
    expect(
      collectExportNames(`export default function Button() {}\nexport function Card() {}\n`),
    ).toEqual(['default', 'Card'])
  })

  it('collects export { a as b } lists', () => {
    expect(collectExportNames(`const a = 1\nconst b = 2\nexport { a as Button, b }\n`)).toEqual([
      'Button',
      'b',
    ])
  })
})

describe('buildClientReferenceStubModule', () => {
  it('emits named and default registerClientReference exports', () => {
    const stub = buildClientReferenceStubModule('src/components/ui.tsx', ['default', 'Card'])

    expect(stub).toContain(
      'export default registerClientReference(null, "src/components/ui.tsx", "default")',
    )
    expect(stub).toContain(
      'export const Card = registerClientReference(null, "src/components/ui.tsx", "Card")',
    )
  })
})

describe('component global wrappers', () => {
  it('builds a named-export wrapper that reads the export off the registry module', () => {
    const code = buildGlobalClientComponentWrapper('TheCard', 'components/ui', 'Card')

    expect(code).toContain('Component["Card"]')
    expect(code).toContain('globalThis[\'~clientComponents\']?.["components/ui"]')
  })

  it('builds a namespace wrapper', () => {
    expect(buildGlobalClientNamespaceWrapper('UI', 'components/ui')).toContain(
      'globalThis[\'~clientComponents\']?.["components/ui"]',
    )
  })
})

describe('transformInlineServerActions', () => {
  it('returns null when there are no inline actions', () => {
    expect(
      transformInlineServerActions(`export async function Page() { return null }\n`, 'page'),
    ).toBeNull()
  })

  it('hoists a nested async function declaration with no closure', () => {
    const input = `import { persist } from './db'
export default async function Page() {
  async function save(formData) {
    'use server'
    await persist(formData)
  }
  return save
}
`

    const result = transformInlineServerActions(input, 'src/app/page')
    expect(result).not.toBeNull()
    expect(result!.actionNames[0]).toMatch(/^\$\$ACTION_0_save$/)
    expect(result!.code).toContain('const save = $$ACTION_0_save')
    expect(result!.code).toContain('async function $$ACTION_0_save(formData)')
    expect(result!.code).toContain('await persist(formData)')
    expect(result!.code).not.toContain("'use server'")
    expect(result!.code).toContain(
      'registerServerReference($$ACTION_0_save, "src/app/page", "$$ACTION_0_save")',
    )
  })

  it('binds closed-over identifiers as leading parameters', () => {
    const input = `import { db } from './db'
export default async function Page({ id }) {
  async function like() {
    "use server"
    await db.like(id)
  }
  return like
}
`

    const result = transformInlineServerActions(input, 'page')
    expect(result!.code).toContain('$$ACTION_0_like.bind(null, id)')
    expect(result!.code).toContain('async function $$ACTION_0_like(id)')
    expect(result!.code).toContain('await db.like(id)')
  })

  it('hoists async arrow actions', () => {
    const input = `import { save } from './db'
export default function Page() {
  const action = async (formData) => {
    'use server'
    await save(formData)
  }
  return action
}
`

    const result = transformInlineServerActions(input, 'page')
    expect(result!.code).toContain('const action = $$ACTION_0_anonymous_server_function')
    expect(result!.code).toContain('async function $$ACTION_0_anonymous_server_function(formData)')
  })
})
