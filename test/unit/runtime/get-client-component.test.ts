import type { ComponentInfo, GlobalWithRari } from '@rari/runtime/shared/types'
import {
  installRscChunkLoader,
  pathsMatch,
  requireClientComponent,
} from '@rari/runtime/shared/get-client-component'
import { describe, expect, it, vi } from 'vite-plus/test'
import { isThenable } from '../../helpers/is-thenable'
import { castMock } from '../../helpers/mock-cast'

interface RequiredClientModule {
  default: (props?: Readonly<Record<string, unknown>>) => unknown
}

type GlobalWithChunkLoader = typeof globalThis & {
  __rari_chunk_load__?: (chunkId: string) => Promise<unknown>
}

/* oxlint-disable typescript/prefer-readonly-parameter-types -- ComponentInfo is a mutable client-component registry entry */
function setClientComponents(
  components: Readonly<NonNullable<GlobalWithRari['~clientComponents']>>,
) {
  Reflect.set(globalThis, '~clientComponents', components)
}
/* oxlint-enable typescript/prefer-readonly-parameter-types */

function requireModule(id: string): RequiredClientModule {
  const module: unknown = requireClientComponent(id)
  if (typeof module !== 'object' || module === null || !('default' in module))
    throw new Error('expected client module')

  return castMock<RequiredClientModule>(module)
}

describe('pathsMatch', () => {
  it('matches identical normalized paths', () => {
    expect(pathsMatch('src/components/Foo.tsx', 'src/components/Foo.tsx')).toBe(true)
    expect(pathsMatch('src\\components\\Foo.tsx', 'src/components/Foo.tsx')).toBe(true)
  })

  it('matches path-boundary-aware suffixes', () => {
    expect(pathsMatch('src/components/Foo.tsx', 'components/Foo.tsx')).toBe(true)
    expect(pathsMatch('components/Foo.tsx', 'src/components/Foo.tsx')).toBe(true)
  })

  it('rejects basename-only matches', () => {
    expect(pathsMatch('src/a/Button.tsx', 'Button.tsx')).toBe(false)
    expect(pathsMatch('src/b/Button.tsx', 'Button.tsx')).toBe(false)
  })

  it('rejects unrelated paths that only share a basename', () => {
    expect(pathsMatch('src/a/Button.tsx', 'src/b/Button.tsx')).toBe(false)
    expect(pathsMatch('src/a/Button.tsx', 'b/Button.tsx')).toBe(false)
  })

  it('rejects partial segment matches without a path boundary', () => {
    expect(pathsMatch('src/components/Foo.tsx', 'Foo.tsx')).toBe(false)
    expect(pathsMatch('src/components/FooBar.tsx', 'Foo.tsx')).toBe(false)
  })
})

describe('requireClientComponent path resolution', () => {
  it('does not resolve ambiguous basename-only component ids', () => {
    const componentA: ComponentInfo = {
      id: 'btn-a',
      path: 'src/a/Button.tsx',
      type: 'client',
      registered: false,
      loader: async () => Promise.resolve({ default: () => null }),
    }
    const componentB: ComponentInfo = {
      id: 'btn-b',
      path: 'src/b/Button.tsx',
      type: 'client',
      registered: false,
      loader: async () => Promise.resolve({ default: () => null }),
    }

    setClientComponents({
      'btn-a': componentA,
      'btn-b': componentB,
    })

    try {
      expect(requireClientComponent('Button.tsx')).toEqual({})
    } finally {
      setClientComponents({})
    }
  })

  it('resolves components by boundary-aware path suffix', () => {
    const componentInfo: ComponentInfo = {
      id: 'btn-a',
      path: 'src/a/Button.tsx',
      type: 'client',
      registered: false,
      loader: async () => Promise.resolve({ default: () => null }),
    }

    setClientComponents({
      'btn-a': componentInfo,
    })

    try {
      const module = requireModule('a/Button.tsx')
      expect(module.default).toBeTypeOf('function')
    } finally {
      setClientComponents({})
    }
  })
})

describe('requireClientComponent lazy load errors', () => {
  it('throws the stored error synchronously after a failed load', async () => {
    const loadError = new Error('network failure')
    const componentInfo: ComponentInfo = {
      id: 'BrokenWidget',
      path: 'src/components/BrokenWidget.tsx',
      type: 'client',
      registered: false,
      loader: async () => Promise.reject(loadError),
    }

    setClientComponents({
      BrokenWidget: componentInfo,
    })

    vi.spyOn(console, 'error').mockImplementation(() => {})

    const suspenseModule = requireModule('BrokenWidget')
    const loadPromise = componentInfo.loadPromise!
    await expect(loadPromise).rejects.toBe(loadError)
    expect(componentInfo.loadError).toBe(loadError)

    const Suspended = suspenseModule.default
    expect(() => Suspended({})).toThrow(loadError)

    const moduleAfterFailure = requireModule('BrokenWidget')
    expect(() => moduleAfterFailure.default({})).toThrow(loadError)

    vi.restoreAllMocks()
    setClientComponents({})
  })

  it('throws the pending load promise while the chunk is loading', () => {
    let resolveLoad!: (value: Readonly<{ default: () => null }>) => void
    const pendingLoad = new Promise<{ default: () => null }>(resolve => {
      resolveLoad = resolve
    })

    const componentInfo: ComponentInfo = {
      id: 'PendingWidget',
      path: 'src/components/PendingWidget.tsx',
      type: 'client',
      registered: false,
      loader: async () => pendingLoad,
    }

    setClientComponents({
      PendingWidget: componentInfo,
    })

    const suspenseModule = requireModule('PendingWidget')
    let thrown: unknown
    try {
      suspenseModule.default({})
    } catch (error) {
      thrown = error
    }

    expect(thrown).toBeInstanceOf(Error)
    expect(thrown).toMatchObject({
      message: '[rari] Lazy component "PendingWidget" is loading',
    })
    expect(isThenable(thrown)).toBe(true)
    expect(componentInfo.loadPromise).toBeDefined()

    resolveLoad({ default: () => null })
    setClientComponents({})
  })

  it('rejects cached chunk load failures from __rari_chunk_load__', async () => {
    vi.stubGlobal('window', {})

    const loadError = new Error('chunk load failed')
    const componentInfo: ComponentInfo = {
      id: 'BrokenChunk',
      path: 'src/components/BrokenChunk.tsx',
      type: 'client',
      registered: false,
      loadError,
      loader: async () => Promise.reject(loadError),
    }

    setClientComponents({
      BrokenChunk: componentInfo,
    })

    try {
      installRscChunkLoader()

      await expect(
        (globalThis as GlobalWithChunkLoader).__rari_chunk_load__?.('BrokenChunk'),
      ).rejects.toBe(loadError)
    } finally {
      vi.unstubAllGlobals()
      setClientComponents({})
    }
  })
})
