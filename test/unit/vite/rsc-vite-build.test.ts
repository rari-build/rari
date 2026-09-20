import { buildRscEntriesWithViteEnvironment } from '@rari/vite/server/rsc-vite-build'
import { describe, expect, it, vi } from 'vite-plus/test'

describe('buildRscEntriesWithViteEnvironment', () => {
  it('returns empty outputs for empty entries', async () => {
    const result = await buildRscEntriesWithViteEnvironment({
      viteBuilder: { environments: {} } as never,
      entries: [],
    })
    expect(result).toEqual({ outputs: new Map(), extraFiles: [] })
  })

  it('returns null when rsc environment is missing', async () => {
    const result = await buildRscEntriesWithViteEnvironment({
      viteBuilder: { environments: {} } as never,
      entries: [{ componentId: 'App', filePath: '/app.tsx' }],
    })
    expect(result).toBeNull()
  })

  it('maps entry chunks from a Vite environment build', async () => {
    const buildConfig = {
      write: false,
      emptyOutDir: true,
      copyPublicDir: true,
      minify: false,
      emitAssets: false,
      rolldownOptions: {},
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
      viteBuilder: {
        environments: {
          rsc: {
            init: async () => {},
            config: { build: buildConfig },
          },
        },
        build,
      } as never,
      entries: [{ componentId: 'App', filePath: '/src/App.tsx' }],
    })

    expect(build).toHaveBeenCalledOnce()
    expect(result).not.toBeNull()
    expect(result?.outputs.get('App')).toEqual({
      code: 'export default function App() {}',
      cssAssetSources: ['.x{color:red}'],
    })
    expect(result?.extraFiles).toEqual([])
    expect(buildConfig.write).toBe(false)
    expect(buildConfig.rolldownOptions).toEqual({})
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
      viteBuilder: {
        environments: {
          rsc: {
            init: async () => {},
            config: { build: buildConfig },
          },
        },
        build: vi.fn().mockRejectedValue(new Error('boom')),
      } as never,
      entries: [{ componentId: 'App', filePath: '/src/App.tsx' }],
    })

    expect(result).toBeNull()
    expect(buildConfig.rolldownOptions).toBe(previousOptions)
    expect(warn).toHaveBeenCalled()
    warn.mockRestore()
  })
})
