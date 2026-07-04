import type { ComponentInfo, GlobalWithRari } from '../../../packages/rari/src/runtime/shared/types'
import { describe, expect, it, vi } from 'vite-plus/test'
import { installRscChunkLoader, pathsMatch, requireClientComponent } from '../../../packages/rari/src/runtime/shared/get-client-component'

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

describe('requireClientComponent lazy load errors', () => {
  it('throws the stored error synchronously after a failed load', async () => {
    const loadError = new Error('network failure')
    const componentInfo: ComponentInfo = {
      id: 'BrokenWidget',
      path: 'src/components/BrokenWidget.tsx',
      type: 'client',
      registered: false,
      loader: () => Promise.reject(loadError),
    }

    ;(globalThis as unknown as GlobalWithRari)['~clientComponents'] = {
      BrokenWidget: componentInfo,
    }

    vi.spyOn(console, 'error').mockImplementation(() => {})

    const suspenseModule = requireClientComponent('BrokenWidget')
    const loadPromise = componentInfo.loadPromise!
    await expect(loadPromise).rejects.toBe(loadError)
    expect(componentInfo.loadError).toBe(loadError)

    const Suspended = suspenseModule.default
    expect(() => Suspended({})).toThrow(loadError)

    const moduleAfterFailure = requireClientComponent('BrokenWidget')
    expect(() => moduleAfterFailure.default({})).toThrow(loadError)

    vi.restoreAllMocks()
    ;(globalThis as unknown as GlobalWithRari)['~clientComponents'] = {}
  })

  it('throws the pending load promise while the chunk is loading', () => {
    let resolveLoad!: (value: { default: () => null }) => void
    const pendingLoad = new Promise<{ default: () => null }>((resolve) => {
      resolveLoad = resolve
    })

    const componentInfo: ComponentInfo = {
      id: 'PendingWidget',
      path: 'src/components/PendingWidget.tsx',
      type: 'client',
      registered: false,
      loader: () => pendingLoad,
    }

    ;(globalThis as unknown as GlobalWithRari)['~clientComponents'] = {
      PendingWidget: componentInfo,
    }

    const suspenseModule = requireClientComponent('PendingWidget')
    expect(() => suspenseModule.default({})).toThrow(componentInfo.loadPromise)

    resolveLoad({ default: () => null })
    ;(globalThis as unknown as GlobalWithRari)['~clientComponents'] = {}
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
      loader: () => Promise.reject(loadError),
    }

    ;(globalThis as unknown as GlobalWithRari)['~clientComponents'] = {
      BrokenChunk: componentInfo,
    }

    installRscChunkLoader()

    await expect((globalThis as any).__rari_chunk_load__('BrokenChunk')).rejects.toBe(loadError)

    vi.unstubAllGlobals()
    ;(globalThis as unknown as GlobalWithRari)['~clientComponents'] = {}
  })
})
