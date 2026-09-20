import type { ViteBuilder } from 'vite-plus'
import { buildRscEntriesWithViteEnvironment } from '@rari/vite/server/rsc-vite-build'
import { describe, expect, it, vi } from 'vite-plus/test'
import { castMock } from '../../helpers/mock-cast'

describe('buildRscEntriesWithViteEnvironment', () => {
  it('returns empty outputs for empty entries', async () => {
    const result = await buildRscEntriesWithViteEnvironment({
      viteBuilder: castMock<ViteBuilder>({ environments: {} }),
      entries: [],
    })
    expect(result).toEqual({ outputs: new Map(), extraFiles: [] })
  })

  it('returns null when rsc environment is missing', async () => {
    const result = await buildRscEntriesWithViteEnvironment({
      viteBuilder: castMock<ViteBuilder>({ environments: {} }),
      entries: [{ componentId: 'App', filePath: '/app.tsx' }],
    })
    expect(result).toBeNull()
  })

  it('maps entry chunks from a Vite environment build', async () => {
    const previousRolldown = { keep: 'me' }
    const buildConfig = {
      write: true,
      emptyOutDir: true,
      copyPublicDir: true,
      minify: 'esbuild' as const,
      emitAssets: false,
      rolldownOptions: previousRolldown,
    }
    const build = vi.fn().mockResolvedValue({
      output: [
        {
          type: 'chunk',
          isEntry: true,
          name: 'App',
          fileName: 'App.js',
          code: 'export default function App() {}',
        },
        {
          type: 'asset',
          fileName: 'assets/App-abc.css',
          source: '.x{color:red}',
        },
      ],
    })

    const result = await buildRscEntriesWithViteEnvironment({
      viteBuilder: castMock<ViteBuilder>({
        environments: {
          rsc: {
            init: async () => {},
            config: { build: buildConfig },
          },
        },
        build,
      }),
      entries: [{ componentId: 'App', filePath: '/src/App.tsx' }],
    })

    expect(build).toHaveBeenCalledOnce()
    expect(result).not.toBeNull()
    expect(result?.outputs.get('App')).toEqual({
      code: 'export default function App() {}',
      cssAssetSources: ['.x{color:red}'],
    })
    expect(result?.extraFiles).toEqual([])
    expect(buildConfig.write).toBe(true)
    expect(buildConfig.emptyOutDir).toBe(true)
    expect(buildConfig.copyPublicDir).toBe(true)
    expect(buildConfig.minify).toBe('esbuild')
    expect(buildConfig.emitAssets).toBe(false)
    expect(buildConfig.rolldownOptions).toBe(previousRolldown)
  })

  it('preserves binary non-css asset sources without utf-8 conversion', async () => {
    const buildConfig = {
      write: false,
      emptyOutDir: false,
      copyPublicDir: false,
      minify: false,
      emitAssets: true,
      rolldownOptions: {},
    }
    const binary = new Uint8Array([0x00, 0xff, 0x80, 0xfe])
    const build = vi.fn().mockResolvedValue({
      output: [
        {
          type: 'chunk',
          isEntry: true,
          name: 'App',
          fileName: 'App.js',
          code: 'export default function App() {}',
        },
        {
          type: 'asset',
          fileName: 'assets/font-abc.woff2',
          source: binary,
        },
      ],
    })

    const result = await buildRscEntriesWithViteEnvironment({
      viteBuilder: castMock<ViteBuilder>({
        environments: {
          rsc: {
            init: async () => {},
            config: { build: buildConfig },
          },
        },
        build,
      }),
      entries: [{ componentId: 'App', filePath: '/src/App.tsx' }],
    })

    expect(result?.extraFiles).toHaveLength(1)
    expect(result?.extraFiles[0]?.fileName).toBe('assets/font-abc.woff2')
    expect(result?.extraFiles[0]?.code).toBe(binary)
  })

  it('returns null and restores config when build throws', async () => {
    const previousOptions = { keep: true }
    const buildConfig = {
      write: false,
      emptyOutDir: true,
      copyPublicDir: true,
      minify: false,
      emitAssets: false,
      rolldownOptions: previousOptions,
    }
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})

    const result = await buildRscEntriesWithViteEnvironment({
      viteBuilder: castMock<ViteBuilder>({
        environments: {
          rsc: {
            init: async () => {},
            config: { build: buildConfig },
          },
        },
        build: vi.fn().mockRejectedValue(new Error('boom')),
      }),
      entries: [{ componentId: 'App', filePath: '/src/App.tsx' }],
    })

    expect(result).toBeNull()
    expect(buildConfig.rolldownOptions).toBe(previousOptions)
    expect(warn).toHaveBeenCalled()
    warn.mockRestore()
  })
})
