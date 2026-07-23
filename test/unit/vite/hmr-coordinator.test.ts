/* oxlint-disable typescript/no-unsafe-assignment -- vitest asymmetric matchers (expect.*) are typed as any */

import type { ModuleAnalysis } from '@rari/vite/analysis/directives'
import type { ComponentRebuildResult } from '@rari/vite/server/build'
import type { ModuleNode, ViteDevServer } from 'vite-plus'
import fs from 'node:fs'
import { analyzeModuleSource } from '@rari/vite/analysis/directives'
import { HMRCoordinator } from '@rari/vite/hmr/coordinator'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import { castMock } from '../../helpers/mock-cast'

vi.mock('node:fs')
vi.mock('@rari/shared/http', () => ({
  throwIfNotOk: vi.fn(),
}))

interface MockServerComponentBuilder {
  rebuildComponent: (filePath: string) => Promise<ComponentRebuildResult>
  getImportGraph: () => ReadonlyMap<string, ReadonlySet<string>>
  invalidateBuildCacheFor: (filePath: string) => void
  getModuleAnalysis: (filePath: string, source?: string) => ModuleAnalysis
}

interface MockFetchResponse {
  ok: boolean
  status?: number
  json: () => Promise<unknown>
  text: () => Promise<string>
}

interface HmrErrorEventPayload {
  event: string
  data: { count: number }
}

function isHmrErrorEventPayload(value: unknown): value is HmrErrorEventPayload {
  return (
    typeof value === 'object' &&
    value !== null &&
    'event' in value &&
    typeof value.event === 'string' &&
    'data' in value &&
    typeof value.data === 'object' &&
    value.data !== null &&
    'count' in value.data &&
    typeof value.data.count === 'number'
  )
}

describe('HMRCoordinator', () => {
  const TEST_DEBOUNCE_MS = 300
  const TEST_PORT = Number(
    process.env.PORT != null && process.env.PORT !== '' ? process.env.PORT : 3000,
  )

  let coordinator: HMRCoordinator
  let mockBuilder: MockServerComponentBuilder
  let mockServer: ViteDevServer
  // Kept as standalone typed mocks (rather than read back off `mockServer`) so
  // assertions never take an unbound reference to a `ViteDevServer` method.
  let getModuleByIdMock: ReturnType<typeof vi.fn<(id: string) => ModuleNode | undefined>>
  let invalidateModuleMock: ReturnType<typeof vi.fn<(mod: ModuleNode) => void>>
  let hotSendMock: ReturnType<typeof vi.fn<(event: string, payload?: unknown) => void>>
  let wsSendMock: ReturnType<typeof vi.fn<(payload: unknown) => void>>
  let mockFetch: ReturnType<
    typeof vi.fn<(...args: readonly unknown[]) => Promise<MockFetchResponse>>
  >
  let originalFetch: typeof globalThis.fetch

  beforeEach(() => {
    vi.clearAllMocks()
    vi.useFakeTimers()

    mockBuilder = {
      rebuildComponent: vi.fn(),
      getImportGraph: vi.fn(() => new Map<string, Set<string>>()),
      invalidateBuildCacheFor: vi.fn<(filePath: string) => void>(),
      getModuleAnalysis: vi.fn((filePath: string, source?: string) => {
        const code = source ?? fs.readFileSync(filePath, 'utf-8')
        return analyzeModuleSource(code)
      }),
    }

    getModuleByIdMock = vi.fn()
    invalidateModuleMock = vi.fn()
    hotSendMock = vi.fn()
    wsSendMock = vi.fn()

    mockServer = castMock<ViteDevServer>({
      moduleGraph: {
        getModuleById: getModuleByIdMock,
        invalidateModule: invalidateModuleMock,
      },
      hot: {
        send: hotSendMock,
      },
      ws: {
        send: wsSendMock,
      },
    })

    originalFetch = globalThis.fetch
    mockFetch = vi.fn<(...args: readonly unknown[]) => Promise<MockFetchResponse>>()
    // @ts-expect-error partial fetch mock for tests
    globalThis.fetch = mockFetch

    // @ts-expect-error partial ServerComponentBuilder mock for tests
    coordinator = new HMRCoordinator(mockBuilder, TEST_PORT)
  })

  afterEach(() => {
    coordinator.dispose()
    globalThis.fetch = originalFetch
    vi.restoreAllMocks()
    vi.useRealTimers()
  })

  describe('handleClientComponentUpdate', () => {
    it('should invalidate module in module graph', async () => {
      const filePath = '/test/src/components/Button.tsx'
      const mockModule = castMock<ModuleNode>({ id: filePath })

      getModuleByIdMock.mockReturnValue(mockModule)

      await coordinator.handleClientComponentUpdate(filePath, mockServer)

      expect(getModuleByIdMock).toHaveBeenCalledWith(filePath)
      expect(invalidateModuleMock).toHaveBeenCalledWith(mockModule)
    })

    it('should handle missing module gracefully', async () => {
      const filePath = '/test/src/components/Missing.tsx'

      getModuleByIdMock.mockReturnValue(undefined)

      await coordinator.handleClientComponentUpdate(filePath, mockServer)

      expect(invalidateModuleMock).not.toHaveBeenCalled()
    })

    it('should handle errors during module invalidation', async () => {
      const filePath = '/test/src/components/Error.tsx'
      const mockModule = castMock<ModuleNode>({ id: filePath })

      getModuleByIdMock.mockReturnValue(mockModule)
      invalidateModuleMock.mockImplementation(() => {
        throw new Error('Invalidation failed')
      })

      await coordinator.handleClientComponentUpdate(filePath, mockServer)

      expect(coordinator.getErrorCount()).toBeGreaterThan(0)
    })
  })

  describe('handleServerComponentUpdate', () => {
    it('should rebuild server component', async () => {
      const filePath1 = '/test/src/components/Server1.tsx'
      const filePath2 = '/test/src/components/Server2.tsx'

      vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
        success: true,
        componentId: 'server-component',
        bundlePath: '/dist/server.js',
      })

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => Promise.resolve({ success: true }),
        text: async () => Promise.resolve(JSON.stringify({ success: true })),
      })

      await coordinator.handleServerComponentUpdate(filePath1, mockServer)
      await coordinator.handleServerComponentUpdate(filePath2, mockServer)

      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(mockBuilder.rebuildComponent).toHaveBeenCalledTimes(2)
    })

    it('should notify rust server on successful rebuild', async () => {
      const filePath = '/test/src/components/Test.tsx'
      const componentId = 'test-component'
      const bundlePath = '/dist/test.js'

      vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
        success: true,
        componentId,
        bundlePath,
      })

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => Promise.resolve({ success: true }),
        text: async () => Promise.resolve(JSON.stringify({ success: true })),
      })

      await coordinator.handleServerComponentUpdate(filePath, mockServer)

      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(mockFetch).toHaveBeenCalledWith(
        `http://localhost:${TEST_PORT}/_rari/hmr`,
        expect.objectContaining({
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            action: 'reload-component',
            component_id: componentId,
            bundle_path: bundlePath,
          }),
        }),
      )
    })

    it('should send HMR update to client on success', async () => {
      const filePath = '/test/src/components/Success.tsx'
      const componentId = 'success-component'

      vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
        success: true,
        componentId,
        bundlePath: '/dist/success.js',
      })

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => Promise.resolve({ success: true }),
        text: async () => Promise.resolve(JSON.stringify({ success: true })),
      })

      await coordinator.handleServerComponentUpdate(filePath, mockServer)

      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(hotSendMock).toHaveBeenCalledWith(
        'rari:server-component-updated',
        expect.objectContaining({
          id: componentId,
          t: expect.any(Number),
        }),
      )
    })

    it('should send error event on rebuild failure', async () => {
      const filePath = '/test/src/components/Fail.tsx'

      vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
        success: false,
        componentId: 'fail-component',
        bundlePath: '',
        error: 'Build failed',
      })

      await coordinator.handleServerComponentUpdate(filePath, mockServer)

      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(wsSendMock).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'custom',
          event: 'rari:hmr-error',
          data: expect.objectContaining({
            msg: expect.stringContaining('Build failed'),
          }),
        }),
      )
    })

    it('should handle rust server notification failure', async () => {
      const filePath = '/test/src/components/ServerFail.tsx'

      vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
        success: true,
        componentId: 'test',
        bundlePath: '/dist/test.js',
      })

      mockFetch.mockRejectedValue(new Error('Server not available'))

      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      await coordinator.handleServerComponentUpdate(filePath, mockServer)

      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(consoleErrorSpy).toHaveBeenCalled()

      consoleErrorSpy.mockRestore()
    })

    it('should clear error state on successful update', async () => {
      const filePath = '/test/src/components/Recovery.tsx'

      vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
        success: false,
        componentId: 'recovery',
        bundlePath: '',
        error: 'Initial build failed',
      })

      await coordinator.handleServerComponentUpdate(filePath, mockServer)
      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(wsSendMock).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'custom',
          event: 'rari:hmr-error',
          data: expect.objectContaining({
            msg: expect.stringContaining('Initial build failed'),
          }),
        }),
      )

      wsSendMock.mockClear()

      vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
        success: true,
        componentId: 'recovery',
        bundlePath: '/dist/recovery.js',
      })

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => Promise.resolve({ success: true }),
        text: async () => Promise.resolve(JSON.stringify({ success: true })),
      })

      await coordinator.handleServerComponentUpdate(filePath, mockServer)
      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(wsSendMock).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'custom',
          event: 'rari:hmr-error-cleared',
        }),
      )
    })
  })

  describe('detectComponentType', () => {
    it('should detect client component', () => {
      const filePath = '/test/src/components/Client.tsx'
      const code = `'use client'

export default function ClientComponent() {
  return <div>Client</div>
}`

      vi.mocked(fs.readFileSync).mockReturnValue(code)

      const type = coordinator.detectComponentType(filePath)

      expect(type).toBe('client')
    })

    it('should detect server component', () => {
      const filePath = '/test/src/components/Server.tsx'
      const code = `export default function ServerComponent() {
  return <div>Server</div>
}`

      vi.mocked(fs.readFileSync).mockReturnValue(code)

      const type = coordinator.detectComponentType(filePath)

      expect(type).toBe('server')
    })

    it('should handle double quotes for use client', () => {
      const filePath = '/test/src/components/DoubleQuote.tsx'
      const code = `"use client"

export default function Component() {
  return <div>Test</div>
}`

      vi.mocked(fs.readFileSync).mockReturnValue(code)

      const type = coordinator.detectComponentType(filePath)

      expect(type).toBe('client')
    })

    it('should ignore use client in comments', () => {
      const filePath = '/test/src/components/Comment.tsx'
      const code = `// 'use client'

export default function Component() {
  return <div>Test</div>
}`

      vi.mocked(fs.readFileSync).mockReturnValue(code)

      const type = coordinator.detectComponentType(filePath)

      expect(type).toBe('server')
    })

    it('should return unknown on file read error', () => {
      const filePath = '/test/src/components/Error.tsx'

      vi.mocked(mockBuilder.getModuleAnalysis).mockImplementation(() => {
        throw new Error('File not found')
      })

      const type = coordinator.detectComponentType(filePath)

      expect(type).toBe('unknown')
    })

    it('should detect use client with semicolon', () => {
      const filePath = '/test/src/components/Semicolon.tsx'
      const code = `'use client';

export default function Component() {
  return <div>Test</div>
}`

      vi.mocked(fs.readFileSync).mockReturnValue(code)

      const type = coordinator.detectComponentType(filePath)

      expect(type).toBe('client')
    })

    it('should detect use client with complete and incomplete block comments', () => {
      const filePath = '/test/src/components/MixedComments.tsx'
      const code = `/* comment */ 'use client' /*

export default function Component() {
  return <div>Test</div>
}`

      vi.mocked(fs.readFileSync).mockReturnValue(code)

      const type = coordinator.detectComponentType(filePath)

      expect(type).toBe('client')
    })

    it('should detect use client with multiple inline block comments', () => {
      const filePath = '/test/src/components/MultipleComments.tsx'
      const code = `/* comment1 */ 'use client' /* comment2 */ /* comment3 */

export default function Component() {
  return <div>Test</div>
}`

      vi.mocked(fs.readFileSync).mockReturnValue(code)

      const type = coordinator.detectComponentType(filePath)

      expect(type).toBe('client')
    })
  })

  describe('dispose', () => {
    it('should clear all pending timers', async () => {
      const filePath = '/test/src/components/Pending.tsx'

      vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
        success: true,
        componentId: 'pending',
        bundlePath: '/dist/pending.js',
      })

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => Promise.resolve({ success: true }),
        text: async () => Promise.resolve(JSON.stringify({ success: true })),
      })

      await coordinator.handleServerComponentUpdate(filePath, mockServer)

      coordinator.dispose()

      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(mockBuilder.rebuildComponent).not.toHaveBeenCalled()

      expect(() => {
        coordinator.dispose()
      }).not.toThrow()
    })

    it('should flush pending logs', async () => {
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})

      await coordinator.handleClientComponentUpdate('/test.tsx', mockServer)

      coordinator.dispose()

      expect(consoleSpy).toHaveBeenCalled()
      expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('[rari] HMR:'))

      consoleSpy.mockRestore()
    })
  })

  describe('error handling', () => {
    it('should track error count', async () => {
      const filePath = '/test/src/components/MultiError.tsx'

      for (let i = 0; i < 3; i++) {
        vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
          success: false,
          componentId: 'error',
          bundlePath: '',
          error: `Error ${i}`,
        })

        await coordinator.handleServerComponentUpdate(filePath, mockServer)
        await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)
      }

      expect(coordinator.getErrorCount()).toBe(3)

      const errorCalls = wsSendMock.mock.calls
        .map(call => call[0])
        .filter(isHmrErrorEventPayload)
        .filter(payload => payload.event === 'rari:hmr-error')

      expect(errorCalls.length).toBe(3)

      const lastErrorCall = errorCalls.at(-1)
      expect(lastErrorCall?.data.count).toBe(3)
    })

    it('should handle invalid server response', async () => {
      const filePath = '/test/src/components/InvalidResponse.tsx'

      vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
        success: true,
        componentId: 'test',
        bundlePath: '/dist/test.js',
      })

      mockFetch.mockResolvedValue({
        ok: true,
        text: async () => Promise.resolve('invalid json'),
        json: async () => Promise.reject(new Error('Invalid JSON')),
      })

      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      await coordinator.handleServerComponentUpdate(filePath, mockServer)
      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(consoleErrorSpy).toHaveBeenCalled()

      consoleErrorSpy.mockRestore()
    })

    it('should handle concurrent updates for different files', async () => {
      const files = ['/test/src/A.tsx', '/test/src/B.tsx', '/test/src/C.tsx']

      vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
        success: true,
        componentId: 'test',
        bundlePath: '/dist/test.js',
      })

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => Promise.resolve({ success: true }),
        text: async () => Promise.resolve(JSON.stringify({ success: true })),
      })

      await Promise.all(
        files.map(async file => coordinator.handleServerComponentUpdate(file, mockServer)),
      )

      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(mockBuilder.rebuildComponent).toHaveBeenCalledTimes(3)
    })

    it('should reset debounce timer on new update', async () => {
      const filePath = '/test/src/components/Debounce.tsx'

      vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
        success: true,
        componentId: 'test',
        bundlePath: '/dist/test.js',
      })

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => Promise.resolve({ success: true }),
        text: async () => Promise.resolve(JSON.stringify({ success: true })),
      })

      await coordinator.handleServerComponentUpdate(filePath, mockServer)

      await vi.advanceTimersByTimeAsync(100)

      await coordinator.handleServerComponentUpdate(filePath, mockServer)

      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(mockBuilder.rebuildComponent).toHaveBeenCalledTimes(1)
    })
  })
})
