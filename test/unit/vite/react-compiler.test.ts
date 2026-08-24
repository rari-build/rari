import type { Plugin } from 'vite-plus'
import { rari } from '@rari/vite'
import {
  createReactCompilerPlugin,
  matchesCompilerId,
  REACT_COMPILER_CODE_FILTER,
  shouldApplyReactCompiler,
} from '@rari/vite/transform/react-compiler'
import { describe, expect, it, vi } from 'vite-plus/test'
import { castMock } from '../../helpers/mock-cast'

const COMPONENT_SOURCE = `
export function Hello({ name }: { name: string }) {
  return <div>Hello {name}</div>
}
`

const PLAIN_SOURCE = `
export const answer = 42
export function getAnswer() {
  return answer
}
`

const ANNOTATED_SOURCE = `
export function Hello({ name }: { name: string }) {
  "use memo"
  return <div>Hello {name}</div>
}
`

type TransformHook = (
  this: Readonly<{
    readonly environment: { readonly config: { readonly consumer: string } }
    readonly error: (message: string) => never
    readonly warn: (message: string) => void
  }>,
  code: string,
  id: string,
) => Promise<{ code?: string; map?: unknown } | null>

type ConfigHook = (
  this: Readonly<{ readonly error: (message: string) => never }>,
  config: Readonly<Record<string, unknown>>,
  env: Readonly<{ command: 'serve' | 'build' }>,
) => Promise<Record<string, unknown> | undefined>

function getTransform(plugin: Plugin): TransformHook {
  const hook = plugin.transform
  if (typeof hook === 'function') {
    return async function (this, code, id) {
      return castMock(await hook.call(castMock(this), code, id))
    }
  }
  if (hook != null && typeof hook === 'object' && typeof hook.handler === 'function') {
    return async function (this, code, id) {
      return castMock(await hook.handler.call(castMock(this), code, id))
    }
  }
  throw new Error('expected transform hook on react-compiler plugin')
}

function getConfig(plugin: Plugin): ConfigHook {
  const hook = plugin.config
  if (typeof hook === 'function') {
    return async function (this, config, env) {
      return castMock(await hook.call(castMock(this), castMock(config), castMock(env)))
    }
  }
  throw new Error('expected config hook on react-compiler plugin')
}

function createClientContext(warn = vi.fn()) {
  return {
    environment: { config: { consumer: 'client' } },
    error(message: string): never {
      throw new Error(message)
    },
    warn,
  }
}

function createServerContext() {
  return {
    environment: { config: { consumer: 'server' } },
    error(message: string): never {
      throw new Error(message)
    },
    warn: vi.fn(),
  }
}

describe('react compiler helpers', () => {
  it('matches component and hook-shaped identifiers', () => {
    expect(REACT_COMPILER_CODE_FILTER.test('export function Hello() {}')).toBe(true)
    expect(REACT_COMPILER_CODE_FILTER.test('const x = useState(0)')).toBe(true)
    expect(REACT_COMPILER_CODE_FILTER.test('memo(Foo)')).toBe(true)
    expect(REACT_COMPILER_CODE_FILTER.test('forwardRef(Foo)')).toBe(true)
    expect(REACT_COMPILER_CODE_FILTER.test('export const answer = 42')).toBe(false)
  })

  it('applies infer mode via the default code filter', () => {
    expect(shouldApplyReactCompiler(COMPONENT_SOURCE, {})).toBe(true)
    expect(shouldApplyReactCompiler(PLAIN_SOURCE, {})).toBe(false)
  })

  it('applies annotation mode only with use memo', () => {
    expect(shouldApplyReactCompiler(COMPONENT_SOURCE, { compilationMode: 'annotation' })).toBe(
      false,
    )
    expect(shouldApplyReactCompiler(ANNOTATED_SOURCE, { compilationMode: 'annotation' })).toBe(true)
  })

  it('forces React 19 compiler runtime regardless of requested target', async () => {
    const plugin = createReactCompilerPlugin({ target: '18' })
    const result = await getConfig(plugin).call(
      {
        error(message: string): never {
          throw new Error(message)
        },
      },
      {},
      { command: 'build' },
    )

    expect(result?.optimizeDeps).toEqual({ include: ['react/compiler-runtime'] })
  })

  it('filters module ids like plugin-react include/exclude', () => {
    expect(matchesCompilerId('/app/src/Hello.tsx')).toBe(true)
    expect(matchesCompilerId('/app/src/Hello.tsx?v=1')).toBe(true)
    expect(matchesCompilerId('/app/src/util.ts')).toBe(true)
    expect(matchesCompilerId('/app/node_modules/react/index.js')).toBe(false)
    expect(matchesCompilerId('/app/src/styles.css')).toBe(false)
    expect(matchesCompilerId('virtual:app-router-provider.tsx')).toBe(false)
    expect(matchesCompilerId('\0virtual:/app/src/Hello.tsx')).toBe(false)
    expect(matchesCompilerId('\0ssr-virtual:/app/src/Hello.tsx')).toBe(false)
  })
})

describe('createReactCompilerPlugin', () => {
  it('disables vite oxc refresh and prebundles the compiler runtime', async () => {
    const plugin = createReactCompilerPlugin(true)
    const result = await getConfig(plugin).call(
      {
        error(message: string): never {
          throw new Error(message)
        },
      },
      {},
      { command: 'serve' },
    )

    expect(result).toEqual({
      oxc: { jsx: { refresh: false } },
      optimizeDeps: { include: ['react/compiler-runtime'] },
    })
  })

  it('compiles client components with oxc-transform-react', async () => {
    const plugin = createReactCompilerPlugin(true)
    const result = await getTransform(plugin).call(
      createClientContext(),
      COMPONENT_SOURCE,
      '/app/src/Hello.tsx',
    )

    expect(result).not.toBeNull()
    expect(result?.code).toContain('react/compiler-runtime')
    expect(result?.code).toContain('react/jsx-runtime')
    expect(result?.code).not.toContain('<div>')
  })

  it('transforms jsx without compiling non-matching client modules', async () => {
    const plugin = createReactCompilerPlugin(true)
    const result = await getTransform(plugin).call(
      createClientContext(),
      PLAIN_SOURCE,
      '/app/src/util.ts',
    )

    expect(result).not.toBeNull()
    expect(result?.code).not.toContain('react/compiler-runtime')
    expect(result?.code).toContain('export const answer = 42')
  })

  it('respects annotation mode', async () => {
    const plugin = createReactCompilerPlugin({ compilationMode: 'annotation' })
    const transform = getTransform(plugin)

    const withoutDirective = await transform.call(
      createClientContext(),
      COMPONENT_SOURCE,
      '/app/src/Hello.tsx',
    )
    expect(withoutDirective?.code).not.toContain('react/compiler-runtime')

    const withDirective = await transform.call(
      createClientContext(),
      ANNOTATED_SOURCE,
      '/app/src/Hello.tsx',
    )
    expect(withDirective?.code).toContain('react/compiler-runtime')
  })

  it('skips server environments', async () => {
    const plugin = createReactCompilerPlugin(true)
    const result = await getTransform(plugin).call(
      createServerContext(),
      COMPONENT_SOURCE,
      '/app/src/Hello.tsx',
    )

    expect(result).toBeNull()
  })

  it('skips node_modules ids', async () => {
    const plugin = createReactCompilerPlugin(true)
    const result = await getTransform(plugin).call(
      createClientContext(),
      COMPONENT_SOURCE,
      '/app/node_modules/ui/Hello.tsx',
    )

    expect(result).toBeNull()
  })

  it('strips vite query strings before transforming', async () => {
    const plugin = createReactCompilerPlugin(true)
    const result = await getTransform(plugin).call(
      createClientContext(),
      COMPONENT_SOURCE,
      '/app/src/Hello.tsx?v=123',
    )

    expect(result?.code).toContain('react/compiler-runtime')
  })
})

describe('rari({ compiler })', () => {
  it('registers the react-compiler plugin when enabled', () => {
    const plugins = rari({ compiler: true })
    expect(plugins.some(plugin => plugin.name === 'rari:react-compiler')).toBe(true)
  })

  it('omits the react-compiler plugin by default', () => {
    const plugins = rari()
    expect(plugins.some(plugin => plugin.name === 'rari:react-compiler')).toBe(false)
  })
})

describe('rari react resolve aliases', () => {
  it('uses an exact react match and pins react/compiler-runtime', async () => {
    const main = rari().find(plugin => plugin.name === 'rari')
    expect(main).toBeDefined()

    const config: {
      resolve?: {
        alias?: Array<{ find: string | RegExp; replacement: string }>
      }
    } = { resolve: {} }

    await getConfig(castMock<Plugin>(main)).call(
      {
        error(message: string): never {
          throw new Error(message)
        },
      },
      castMock(config),
      { command: 'build' },
    )

    const aliases = config.resolve?.alias ?? []
    const reactAlias = aliases.find(
      (entry): entry is { find: RegExp; replacement: string } =>
        entry.find instanceof RegExp && entry.find.source === '^react$' && entry.find.flags === '',
    )
    expect(reactAlias).toBeDefined()
    expect(reactAlias!.find.test('react')).toBe(true)
    expect(reactAlias!.find.test('react/compiler-runtime')).toBe(false)
    expect(aliases.some(entry => entry.find === 'react')).toBe(false)
    expect(aliases.some(entry => entry.find === 'react/compiler-runtime')).toBe(true)
  })
})
