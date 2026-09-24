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

  it('ignores export-like text in comments and strings', () => {
    expect(
      collectExportNames(`
// export function Fake() {}
/* export default function AlsoFake() {} */
const msg = "export function Nope() {}"
export function Real() {}
`),
    ).toEqual(['Real'])
  })

  it('collects export * as namespace names', () => {
    expect(collectExportNames(`export * as ns from './mod'\nexport function Real() {}\n`)).toEqual([
      'ns',
      'Real',
    ])
  })

  it('ignores bare export * re-exports', () => {
    expect(collectExportNames(`export * from './mod'\n`)).toEqual([])
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

  it('ignores string literals and comments when collecting free vars', () => {
    const input = `import { db } from './db'
export default async function Page() {
  async function save(formData) {
    'use server'
    // mentions leakedVar in a comment
    await db.write(formData, "alsoLeaked")
  }
  return save
}
`

    const result = transformInlineServerActions(input, 'page')
    expect(result!.code).toContain('async function $$ACTION_0_save(formData)')
    expect(result!.code).not.toContain('.bind(null,')
  })

  it('emits export default for a directly default-exported inline action', () => {
    const input = `import { db } from './db'
export default async function save(formData) {
  'use server'
  await db.write(formData)
}
`

    const result = transformInlineServerActions(input, 'page')
    expect(result).not.toBeNull()
    expect(result!.actionNames[0]).toMatch(/^\$\$ACTION_0_save$/)
    expect(result!.rewrittenExportNames).toEqual(['default'])
    expect(result!.code).toContain(
      'export default registerServerReference($$ACTION_0_save, "page", "default")',
    )
    expect(result!.code).toContain('async function $$ACTION_0_save(formData)')
    expect(result!.code).not.toContain('const save =')
    expect(result!.code).not.toContain(
      'registerServerReference($$ACTION_0_save, "page", "$$ACTION_0_save")',
    )
  })

  it('preserves surrounding binding for exported arrow actions and registers once', () => {
    const input = `import { db } from './db'
export const save = async (formData) => {
  'use server'
  await db.write(formData)
}
`

    const result = transformInlineServerActions(input, 'page')
    expect(result!.rewrittenExportNames).toEqual(['save'])
    expect(result!.code).toContain(
      'export const save = registerServerReference($$ACTION_0_anonymous_server_function, "page", "save")',
    )
    expect(result!.code).toContain('async function $$ACTION_0_anonymous_server_function(formData)')
    expect(result!.code).not.toContain(
      'registerServerReference($$ACTION_0_anonymous_server_function, "page", "$$ACTION_0_anonymous_server_function")',
    )
  })

  it('treats named function expressions as expressions under assignment', () => {
    const input = `import { db } from './db'
export const save = async function named(formData) {
  'use server'
  await db.write(formData)
}
`

    const result = transformInlineServerActions(input, 'page')
    expect(result!.rewrittenExportNames).toEqual(['save'])
    expect(result!.code).toContain(
      'export const save = registerServerReference($$ACTION_0_named, "page", "save")',
    )
    expect(result!.code).toContain('async function $$ACTION_0_named(formData)')
    expect(result!.code).not.toContain('export const named =')
    expect(result!.code).not.toContain('const save = const named')
  })

  it('locates single-parameter async arrow actions', () => {
    const input = `import { save } from './db'
export default function Page() {
  const action = async formData => {
    'use server'
    await save(formData)
  }
  return action
}
`

    const result = transformInlineServerActions(input, 'page')
    expect(result).not.toBeNull()
    expect(result!.code).toContain('const action = $$ACTION_0_anonymous_server_function')
    expect(result!.code).toContain('async function $$ACTION_0_anonymous_server_function(formData)')
  })

  it('captures enclosing vars that collide with destructured property keys', () => {
    const input = `import { db } from './db'
export default async function Page({ id }) {
  async function like({ id: ignored }) {
    'use server'
    await db.like(id)
  }
  return like
}
`

    const result = transformInlineServerActions(input, 'page')
    expect(result!.code).toContain('$$ACTION_0_like.bind(null, id)')
    expect(result!.code).toContain('async function $$ACTION_0_like(id, { id: ignored })')
  })

  it('captures enclosing vars used only via object spread', () => {
    const input = `import { db } from './db'
export default async function Page({ defaults }) {
  async function save(formData) {
    'use server'
    await db.write({ ...defaults })
  }
  return save
}
`

    const result = transformInlineServerActions(input, 'page')
    expect(result!.code).toContain('$$ACTION_0_save.bind(null, defaults)')
    expect(result!.code).toContain('async function $$ACTION_0_save(defaults, formData)')
    expect(result!.code).toContain('...defaults')
  })

  it('does not treat default-value identifiers as destructured bindings', () => {
    const input = `import { db } from './db'
export default async function Page({ fallback }) {
  async function save({ count = fallback }) {
    'use server'
    await db.write(count)
  }
  return save
}
`

    const result = transformInlineServerActions(input, 'page')
    expect(result!.code).toContain('$$ACTION_0_save.bind(null, fallback)')
    expect(result!.code).toContain('async function $$ACTION_0_save(fallback, { count = fallback })')
  })

  it('keeps named functions in return position as expressions', () => {
    const input = `import { db } from './db'
export default function Page() {
  return async function save(formData) {
    'use server'
    await db.write(formData)
    if (!formData) return save(new FormData())
  }
}
`

    const result = transformInlineServerActions(input, 'page')
    expect(result!.code).toContain('return $$ACTION_0_save')
    expect(result!.code).toContain('async function $$ACTION_0_save(formData)')
    expect(result!.code).toContain('return $$ACTION_0_save(new FormData())')
    expect(result!.code).not.toContain('$$ACTION_0_save.bind(')
    expect(result!.code).not.toContain('const save =')
    expect(result!.code).not.toContain('return const ')
  })

  it('keeps named functions in call-argument position as expressions', () => {
    const input = `import { db } from './db'
import { register } from './register'
export default function Page() {
  register(async function save(formData) {
    'use server'
    await db.write(formData)
  })
}
`

    const result = transformInlineServerActions(input, 'page')
    expect(result!.code).toContain('register($$ACTION_0_save)')
    expect(result!.code).toContain('async function $$ACTION_0_save(formData)')
    expect(result!.code).not.toContain('const save =')
    expect(result!.code).not.toContain('register(const ')
  })

  it('keeps named functions in JSX action attributes as expressions', () => {
    const input = `import { db } from './db'
export default function Page() {
  return <form action={async function save(formData) {
    'use server'
    await db.write(formData)
  }} />
}
`

    const result = transformInlineServerActions(input, 'page')
    expect(result).not.toBeNull()
    expect(result!.code).toContain('action={$$ACTION_0_save}')
    expect(result!.code).toContain('async function $$ACTION_0_save(formData)')
    expect(result!.code).not.toContain('action={const ')
    expect(result!.code).not.toContain('const save =')
    const attr = /action=\{([^}]+)\}/.exec(result!.code)
    expect(attr?.[1]).toMatch(/^\$\$ACTION_\d+_\w+$/)
  })
})
