import type { ViteDevServer } from 'vite-plus'
import type { ServerComponentBuilder } from '../server/build'
import path from 'node:path'
import process from 'node:process'
import { throwIfNotOk } from '@/shared/utils/http'
import { isRecord, parseJsonRecord } from '@/shared/utils/type-guards'
import { HMRErrorHandler } from './error-handler'

export interface ComponentRebuildResult {
  componentId: string
  bundlePath: string
  success: boolean
  error?: string
}

type HmrLogType = 'info' | 'success' | 'warning' | 'error'

interface SuccessfulRebuildResult {
  success: true
  componentId: string
  filePath: string
  relativePath: string
}

interface FailedRebuildResult {
  success: false
  filePath: string
  relativePath: string
  error: Error
}

function isSuccessfulRebuildResult(value: unknown): value is SuccessfulRebuildResult {
  return (
    isRecord(value) &&
    value.success === true &&
    typeof value.componentId === 'string' &&
    typeof value.filePath === 'string' &&
    typeof value.relativePath === 'string'
  )
}

function isFailedRebuildResult(value: unknown): value is FailedRebuildResult {
  return (
    isRecord(value) &&
    value.success === false &&
    typeof value.filePath === 'string' &&
    typeof value.relativePath === 'string' &&
    value.error instanceof Error
  )
}

function isHmrLogType(value: string): value is HmrLogType {
  return value === 'info' || value === 'success' || value === 'warning' || value === 'error'
}

export class HMRCoordinator {
  private readonly serverComponentBuilder: ServerComponentBuilder
  private readonly rustServerUrl: string
  private readonly pendingUpdates = new Map<string, NodeJS.Timeout>()
  private readonly pendingFiles = new Set<string>()
  private batchTimer: NodeJS.Timeout | null = null
  private readonly DEBOUNCE_DELAY = 200
  private readonly errorHandler: HMRErrorHandler
  private logBatch: Array<{ type: HmrLogType; message: string; timestamp: number }> = []
  private logBatchTimer: NodeJS.Timeout | null = null
  private readonly LOG_BATCH_DELAY = 500

  constructor(
    // oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- ServerComponentBuilder is a stateful builder with internal mutable caches
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

  getErrorCount(): number {
    return this.errorHandler.getErrorCount()
  }

  async handleClientComponentUpdate(filePath: string, server: ViteDevServer): Promise<void> {
    const relativePath = path.relative(process.cwd(), filePath)

    try {
      const module = server.moduleGraph.getModuleById(filePath)

      if (module) {
        server.moduleGraph.invalidateModule(module)

        this.errorHandler.reset()
      } else {
        this.queueLog('warning', `Client component module not found in graph: ${relativePath}`)
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      this.queueLog('error', `Failed to update client component: ${relativePath} - ${errorMessage}`)
      this.errorHandler.recordError(error instanceof Error ? error : new Error(errorMessage))
    }

    return Promise.resolve()
  }

  async handleServerComponentUpdate(filePath: string, server: ViteDevServer): Promise<void> {
    this.pendingFiles.add(filePath)

    const existingTimer = this.pendingUpdates.get(filePath)
    if (existingTimer) {
      clearTimeout(existingTimer)
      this.pendingUpdates.delete(filePath)
    }

    if (this.batchTimer) clearTimeout(this.batchTimer)

    this.batchTimer = setTimeout(() => {
      void (async () => {
        const filesToProcess = [...this.pendingFiles]
        this.pendingFiles.clear()
        this.batchTimer = null

        if (filesToProcess.length === 0) return

        const dependentPages = this.collectDependentPageComponents(filesToProcess)
        for (const dep of dependentPages) this.serverComponentBuilder.invalidateBuildCacheFor(dep)
        const allFilesToRebuild = [...filesToProcess, ...dependentPages]

        const results = await Promise.allSettled(
          allFilesToRebuild.map(async file => {
            const relativePath = path.relative(process.cwd(), file)

            try {
              const result = await this.serverComponentBuilder.rebuildComponent(file)

              if (!result.success)
                throw new Error(
                  result.error != null && result.error !== '' ? result.error : 'Build failed',
                )

              await this.notifyRustServer(result.componentId, result.bundlePath)

              return {
                success: true,
                componentId: result.componentId,
                filePath: file,
                relativePath,
              }
            } catch (error) {
              return {
                success: false,
                filePath: file,
                relativePath,
                error: error instanceof Error ? error : new Error(String(error)),
              }
            }
          }),
        )

        const successful: Array<{ componentId: string; filePath: string; relativePath: string }> =
          []
        const failed: Array<{ filePath: string; relativePath: string; error: Error }> = []

        results.forEach(result => {
          if (result.status === 'fulfilled' && isSuccessfulRebuildResult(result.value)) {
            successful.push(result.value)
          } else if (result.status === 'fulfilled' && isFailedRebuildResult(result.value)) {
            failed.push(result.value)
          } else if (result.status === 'rejected') {
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
            const errorStack = (
              error.stack != null && error.stack !== '' ? error.stack : ''
            ).substring(0, 500)

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
            this.queueLog(
              'error',
              `Maximum error count reached (${this.errorHandler.getErrorCount()}). ` +
                'Consider restarting the dev server if issues persist.',
            )
          }
        }
      })()
    }, this.DEBOUNCE_DELAY)

    return Promise.resolve()
  }

  private async notifyRustServer(componentId: string, bundlePath: string): Promise<void> {
    try {
      const response = await fetch(`${this.rustServerUrl}/_rari/hmr`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          action: 'reload-component',
          component_id: componentId,
          bundle_path: bundlePath,
        }),
      })

      await throwIfNotOk(response)

      const responseText = await response.text()

      let parsed: unknown
      try {
        parsed = JSON.parse(responseText)
      } catch (parseError) {
        throw new Error(
          `Failed to parse server response (status ${response.status}): ${parseError instanceof Error ? parseError.message : String(parseError)}. ` +
            `Response body: ${responseText.substring(0, 200)}${responseText.length > 200 ? '...' : ''}`,
        )
      }

      const result = parseJsonRecord(responseText)
      if (!result) {
        throw new Error(
          `Invalid server response (status ${response.status}): expected object, got ${typeof parsed}. ` +
            `Response body: ${responseText.substring(0, 200)}${responseText.length > 200 ? '...' : ''}`,
        )
      }

      if (result.success !== true) {
        const message =
          typeof result.message === 'string' && result.message !== ''
            ? result.message
            : typeof result.error === 'string' && result.error !== ''
              ? result.error
              : 'Component reload failed'
        throw new Error(message)
      }
    } catch (error) {
      console.error(`[rari] HMR: Failed to notify Rust server:`, error)
      throw error
    }
  }

  private collectDependentPageComponents(changedFiles: readonly string[]): string[] {
    const importGraph = this.serverComponentBuilder.getImportGraph()
    const dependentPages = new Set<string>()
    const visited = new Set<string>()

    const findPageImporters = (filePath: string) => {
      if (visited.has(filePath)) return
      visited.add(filePath)

      const importers = importGraph.get(filePath)
      if (!importers) return

      for (const importer of importers) {
        const isAppFile = importer.includes('/app/') || importer.includes('\\app\\')
        if (isAppFile && !changedFiles.includes(importer)) {
          dependentPages.add(importer)
        }
        findPageImporters(importer)
      }
    }

    for (const file of changedFiles) findPageImporters(file)

    return [...dependentPages]
  }

  detectComponentType(filePath: string): 'client' | 'server' | 'unknown' {
    try {
      const analysis = this.serverComponentBuilder.getModuleAnalysis(filePath)
      return analysis.topLevelUseClient ? 'client' : 'server'
    } catch {
      return 'unknown'
    }
  }

  private queueLog(type: HmrLogType, message: string): void {
    this.logBatch.push({
      type,
      message,
      timestamp: Date.now(),
    })

    if (this.logBatchTimer) clearTimeout(this.logBatchTimer)

    this.logBatchTimer = setTimeout(() => {
      this.flushLogs()
    }, this.LOG_BATCH_DELAY)
  }

  private flushLogs(): void {
    if (this.logBatch.length === 0) return

    const grouped = this.logBatch.reduce<Record<string, typeof this.logBatch>>((acc, log) => {
      acc[log.type] ??= []
      acc[log.type].push(log)
      return acc
    }, {})

    for (const [type, logs] of Object.entries(grouped)) {
      if (!isHmrLogType(type)) continue

      if (logs.length === 1) {
        const log = logs[0]
        this.outputLog(type, log.message)
      } else {
        const messages = logs.map(l => l.message).join('\n  • ')
        this.outputLog(type, `${logs.length} updates:\n  • ${messages}`)
      }
    }

    this.logBatch = []
    this.logBatchTimer = null
  }

  private outputLog(type: HmrLogType, message: string): void {
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

    for (const timer of this.pendingUpdates.values()) clearTimeout(timer)

    this.pendingUpdates.clear()
    this.pendingFiles.clear()
    this.errorHandler.dispose()
  }
}
