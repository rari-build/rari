import type { ViteDevServer } from 'rolldown-vite'
import type { ServerComponentBuilder } from './server-build'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { HMRErrorHandler } from './hmr-error-handler'

export interface ComponentRebuildResult {
  componentId: string
  bundlePath: string
  success: boolean
  error?: string
}

export class HMRCoordinator {
  private serverComponentBuilder: ServerComponentBuilder
  private rustServerUrl: string
  private pendingUpdates = new Map<string, NodeJS.Timeout>()
  private pendingFiles = new Set<string>()
  private batchTimer: NodeJS.Timeout | null = null
  private readonly DEBOUNCE_DELAY = 200
  private errorHandler: HMRErrorHandler
  private logBatch: Array<{ type: string, message: string, timestamp: number }> = []
  private logBatchTimer: NodeJS.Timeout | null = null
  private readonly LOG_BATCH_DELAY = 500

  constructor(
    builder: ServerComponentBuilder,
    serverPort: number = 3000,
  ) {
    this.serverComponentBuilder = builder
    this.rustServerUrl = `http://localhost:${serverPort}`
    this.errorHandler = new HMRErrorHandler({
      maxErrors: 5,
      resetTimeout: 30000,
    })
  }

  async handleClientComponentUpdate(
    filePath: string,
    server: ViteDevServer,
  ): Promise<void> {
    const relativePath = path.relative(process.cwd(), filePath)

    try {
      const module = server.moduleGraph.getModuleById(filePath)

      if (module) {
        server.moduleGraph.invalidateModule(module)

        this.errorHandler.reset()
      }
      else {
        this.queueLog('warning', `Client component module not found in graph: ${relativePath}`)
      }
    }
    catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      this.queueLog('error', `Failed to update client component: ${relativePath} - ${errorMessage}`)
      this.errorHandler.recordError(error instanceof Error ? error : new Error(errorMessage))
    }
  }

  async handleServerComponentUpdate(
    filePath: string,
    server: ViteDevServer,
  ): Promise<void> {
    this.pendingFiles.add(filePath)

    const existingTimer = this.pendingUpdates.get(filePath)
    if (existingTimer) {
      clearTimeout(existingTimer)
      this.pendingUpdates.delete(filePath)
    }

    if (this.batchTimer)
      clearTimeout(this.batchTimer)

    this.batchTimer = setTimeout(async () => {
      const filesToProcess = [...this.pendingFiles]
      this.pendingFiles.clear()
      this.batchTimer = null

      if (filesToProcess.length === 0)
        return

      const results = await Promise.allSettled(
        filesToProcess.map(async (file) => {
          const relativePath = path.relative(process.cwd(), file)

          try {
            const result = await (this.serverComponentBuilder as any).rebuildComponent(
              file,
            ) as ComponentRebuildResult

            if (!result.success)
              throw new Error(result.error || 'Build failed')

            await this.notifyRustServer(result.componentId, result.bundlePath)

            return {
              success: true,
              componentId: result.componentId,
              filePath: file,
              relativePath,
            }
          }
          catch (error) {
            return {
              success: false,
              filePath: file,
              relativePath,
              error: error instanceof Error ? error : new Error(String(error)),
            }
          }
        }),
      )

      const successful: Array<{ componentId: string, filePath: string, relativePath: string }> = []
      const failed: Array<{ filePath: string, relativePath: string, error: Error }> = []

      results.forEach((result) => {
        if (result.status === 'fulfilled' && result.value.success) {
          successful.push(result.value as any)
        }
        else if (result.status === 'fulfilled' && !result.value.success) {
          failed.push(result.value as any)
        }
        else if (result.status === 'rejected') {
          failed.push({
            filePath: '',
            relativePath: 'unknown',
            error: new Error(String(result.reason)),
          })
        }
      })

      if (successful.length > 0) {
        const timestamp = Date.now()

        successful.forEach(({ componentId }) => {
          server.hot.send('rari:server-component-updated', {
            id: componentId,
            t: timestamp,
          })
        })

        this.errorHandler.reset()

        server.ws.send({
          type: 'custom',
          event: 'rari:hmr-error-cleared',
          data: { t: timestamp },
        })
      }

      if (failed.length > 0) {
        const timestamp = Date.now()

        failed.forEach(({ relativePath, error }) => {
          const errorMessage = error.message
          const errorStack = (error.stack || '').substring(0, 500)

          this.queueLog('error', `Failed to rebuild: ${relativePath} - ${errorMessage}`)

          this.errorHandler.recordError(error)

          server.ws.send({
            type: 'custom',
            event: 'rari:hmr-error',
            data: {
              msg: errorMessage,
              stack: errorStack,
              file: relativePath,
              t: timestamp,
              count: this.errorHandler.getErrorCount(),
              max: 5,
            },
          })
        })

        if (this.errorHandler.hasReachedMaxErrors()) {
          this.queueLog('error', `Maximum error count reached (${this.errorHandler.getErrorCount()}). `
          + 'Consider restarting the dev server if issues persist.')
        }
      }
    }, this.DEBOUNCE_DELAY)
  }

  private async notifyRustServer(
    componentId: string,
    bundlePath: string,
  ): Promise<void> {
    try {
      const response = await fetch(
        `${this.rustServerUrl}/_rari/hmr`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            action: 'reload-component',
            component_id: componentId,
            bundle_path: bundlePath,
          }),
        },
      )

      if (!response.ok) {
        const errorText = await response.text()
        throw new Error(`HTTP ${response.status}: ${errorText}`)
      }

      interface ServerResponse {
        success: boolean
        message?: string
        error?: string
        data?: unknown
      }

      const responseText = await response.text()

      let parsed: unknown
      try {
        parsed = JSON.parse(responseText)
      }
      catch (parseError) {
        throw new Error(
          `Failed to parse server response (status ${response.status}): ${parseError instanceof Error ? parseError.message : String(parseError)}. `
          + `Response body: ${responseText.substring(0, 200)}${responseText.length > 200 ? '...' : ''}`,
        )
      }

      if (!parsed || typeof parsed !== 'object') {
        throw new Error(
          `Invalid server response (status ${response.status}): expected object, got ${typeof parsed}. `
          + `Response body: ${responseText.substring(0, 200)}${responseText.length > 200 ? '...' : ''}`,
        )
      }

      const result = parsed as ServerResponse

      if (!result.success) {
        throw new Error(result.message || result.error || 'Component reload failed')
      }
    }
    catch (error) {
      console.error(`[rari] HMR: Failed to notify Rust server:`, error)
      throw error
    }
  }

  detectComponentType(filePath: string): 'client' | 'server' | 'unknown' {
    try {
      const code = fs.readFileSync(filePath, 'utf-8')

      const lines = code.split('\n')
      for (const line of lines) {
        const trimmed = line.trim()
        if (!trimmed || trimmed.startsWith('//') || trimmed.startsWith('/*'))
          continue
        if (trimmed === '\'use client\'' || trimmed === '"use client"')
          return 'client'
        break
      }

      return 'server'
    }
    catch {
      return 'unknown'
    }
  }

  private queueLog(type: 'info' | 'success' | 'warning' | 'error', message: string): void {
    this.logBatch.push({
      type,
      message,
      timestamp: Date.now(),
    })

    if (this.logBatchTimer)
      clearTimeout(this.logBatchTimer)

    this.logBatchTimer = setTimeout(() => {
      this.flushLogs()
    }, this.LOG_BATCH_DELAY)
  }

  private flushLogs(): void {
    if (this.logBatch.length === 0)
      return

    const grouped = this.logBatch.reduce((acc, log) => {
      if (!acc[log.type])
        acc[log.type] = []
      acc[log.type].push(log)
      return acc
    }, {} as Record<string, typeof this.logBatch>)

    for (const [type, logs] of Object.entries(grouped)) {
      if (logs.length === 1) {
        const log = logs[0]
        this.outputLog(type as any, log.message)
      }
      else {
        const messages = logs.map(l => l.message).join('\n  • ')
        this.outputLog(type as any, `${logs.length} updates:\n  • ${messages}`)
      }
    }

    this.logBatch = []
    this.logBatchTimer = null
  }

  private outputLog(type: 'info' | 'success' | 'warning' | 'error', message: string): void {
    const prefix = '[rari] HMR:'

    switch (type) {
      case 'success':
        console.warn(`\x1B[32m${prefix}\x1B[0m ${message}`)
        break
      case 'warning':
        console.warn(`\x1B[33m${prefix}\x1B[0m ${message}`)
        break
      case 'error':
        console.error(`\x1B[31m${prefix}\x1B[0m ${message}`)
        break
      case 'info':
      default:
        console.warn(`${prefix} ${message}`)
        break
    }
  }

  dispose(): void {
    if (this.logBatchTimer) {
      clearTimeout(this.logBatchTimer)
      this.flushLogs()
    }

    if (this.batchTimer) {
      clearTimeout(this.batchTimer)
      this.batchTimer = null
    }

    for (const timer of this.pendingUpdates.values())
      clearTimeout(timer)

    this.pendingUpdates.clear()
    this.pendingFiles.clear()
    this.errorHandler.dispose()
  }
}
