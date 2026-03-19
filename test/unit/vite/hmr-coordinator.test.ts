import type { ViteDevServer } from 'vite-plus'
import fs from 'node:fs'
import { HMRCoordinator } from '@rari/vite/hmr-coordinator'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'

vi.mock('node:fs')
vi.mock('@rari/shared/http-utils', () => ({
  throwIfNotOk: vi.fn(),
}))

describe('HMRCoordinator', () => {
  const TEST_DEBOUNCE_MS = 300

  let coordinator: HMRCoordinator
  let mockBuilder: any
  let mockServer: ViteDevServer
  let mockFetch: any
  let originalFetch: typeof globalThis.fetch

  beforeEach(() => {
    vi.clearAllMocks()
    vi.useFakeTimers()

    mockBuilder = {
      rebuildComponent: vi.fn(),
    }

    mockServer = {
      moduleGraph: {
        getModuleById: vi.fn(),
        invalidateModule: vi.fn(),
      },
      hot: {
        send: vi.fn(),
      },
      ws: {
        send: vi.fn(),
      },
    } as any

    originalFetch = globalThis.fetch
    mockFetch = vi.fn()
    globalThis.fetch = mockFetch

    coordinator = new HMRCoordinator(mockBuilder, 3000)
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
      const mockModule = { id: filePath }

      vi.mocked(mockServer.moduleGraph.getModuleById).mockReturnValue(mockModule as any)

      await coordinator.handleClientComponentUpdate(filePath, mockServer)

      expect(mockServer.moduleGraph.getModuleById).toHaveBeenCalledWith(filePath)
      expect(mockServer.moduleGraph.invalidateModule).toHaveBeenCalledWith(mockModule)
    })

    it('should handle missing module gracefully', async () => {
      const filePath = '/test/src/components/Missing.tsx'

      vi.mocked(mockServer.moduleGraph.getModuleById).mockReturnValue(undefined)

      await coordinator.handleClientComponentUpdate(filePath, mockServer)

      expect(mockServer.moduleGraph.invalidateModule).not.toHaveBeenCalled()
    })

    it('should handle errors during module invalidation', async () => {
      const filePath = '/test/src/components/Error.tsx'
      const mockModule = { id: filePath }

      vi.mocked(mockServer.moduleGraph.getModuleById).mockReturnValue(mockModule as any)
      vi.mocked(mockServer.moduleGraph.invalidateModule).mockImplementation(() => {
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
        json: async () => ({ success: true }),
        text: async () => JSON.stringify({ success: true }),
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
        json: async () => ({ success: true }),
        text: async () => JSON.stringify({ success: true }),
      })

      await coordinator.handleServerComponentUpdate(filePath, mockServer)

      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:3000/_rari/hmr',
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
        json: async () => ({ success: true }),
        text: async () => JSON.stringify({ success: true }),
      })

      await coordinator.handleServerComponentUpdate(filePath, mockServer)

      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(mockServer.hot.send).toHaveBeenCalledWith(
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

      expect(mockServer.ws.send).toHaveBeenCalledWith(
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

      expect(mockServer.ws.send).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'custom',
          event: 'rari:hmr-error',
          data: expect.objectContaining({
            msg: expect.stringContaining('Initial build failed'),
          }),
        }),
      )

      vi.mocked(mockServer.ws.send).mockClear()

      vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
        success: true,
        componentId: 'recovery',
        bundlePath: '/dist/recovery.js',
      })

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ success: true }),
        text: async () => JSON.stringify({ success: true }),
      })

      await coordinator.handleServerComponentUpdate(filePath, mockServer)
      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(mockServer.ws.send).toHaveBeenCalledWith(
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

      vi.mocked(fs.readFileSync).mockImplementation(() => {
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
  })

  describe('dispose', () => {
    it('should clear all pending timers', () => {
      const filePath = '/test/src/components/Pending.tsx'

      coordinator.handleServerComponentUpdate(filePath, mockServer)

      coordinator.dispose()

      expect(() => coordinator.dispose()).not.toThrow()
    })

    it('should flush pending logs', async () => {
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})

      await coordinator.handleClientComponentUpdate('/test.tsx', mockServer)

      coordinator.dispose()

      expect(consoleSpy).toHaveBeenCalled()
      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining('[rari] HMR:'),
      )

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

      const errorCalls = (mockServer.ws.send as any).mock.calls.filter(
        (call: any) => call[0]?.event === 'rari:hmr-error',
      )

      expect(errorCalls.length).toBe(3)
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
        text: async () => 'invalid json',
        json: async () => {
          throw new Error('Invalid JSON')
        },
      })

      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      await coordinator.handleServerComponentUpdate(filePath, mockServer)
      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(consoleErrorSpy).toHaveBeenCalled()

      consoleErrorSpy.mockRestore()
    })

    it('should handle concurrent updates for different files', async () => {
      const files = [
        '/test/src/A.tsx',
        '/test/src/B.tsx',
        '/test/src/C.tsx',
      ]

      vi.mocked(mockBuilder.rebuildComponent).mockResolvedValue({
        success: true,
        componentId: 'test',
        bundlePath: '/dist/test.js',
      })

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ success: true }),
        text: async () => JSON.stringify({ success: true }),
      })

      await Promise.all(
        files.map(file => coordinator.handleServerComponentUpdate(file, mockServer)),
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
        json: async () => ({ success: true }),
        text: async () => JSON.stringify({ success: true }),
      })

      await coordinator.handleServerComponentUpdate(filePath, mockServer)

      await vi.advanceTimersByTimeAsync(100)

      await coordinator.handleServerComponentUpdate(filePath, mockServer)

      await vi.advanceTimersByTimeAsync(TEST_DEBOUNCE_MS)

      expect(mockBuilder.rebuildComponent).toHaveBeenCalledTimes(1)
    })
  })
})
