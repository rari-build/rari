export interface HMRErrorHandlerOptions {
  maxErrors?: number
  resetTimeout?: number
}

export class HMRErrorHandler {
  private errorCount: number = 0
  private readonly maxErrors: number
  private readonly resetTimeout: number
  private resetTimer: NodeJS.Timeout | null = null
  private lastError: Error | null = null

  constructor(options: HMRErrorHandlerOptions = {}) {
    this.maxErrors = options.maxErrors ?? 5
    this.resetTimeout = options.resetTimeout ?? 30000
  }

  recordError(error: Error): void {
    this.errorCount++
    this.lastError = error

    if (this.resetTimer) {
      clearTimeout(this.resetTimer)
    }

    this.resetTimer = setTimeout(() => {
      this.reset()
    }, this.resetTimeout)

    if (this.errorCount >= this.maxErrors) {
      this.handleMaxErrorsReached()
    }
  }

  reset(): void {
    this.errorCount = 0
    this.lastError = null

    if (this.resetTimer) {
      clearTimeout(this.resetTimer)
      this.resetTimer = null
    }
  }

  getErrorCount(): number {
    return this.errorCount
  }

  getLastError(): Error | null {
    return this.lastError
  }

  hasReachedMaxErrors(): boolean {
    return this.errorCount >= this.maxErrors
  }

  private handleMaxErrorsReached(): void {
    console.error(
      `[HMR] Maximum error count (${this.maxErrors}) reached. `
      + 'Consider restarting the dev server if issues persist.',
    )
  }

  dispose(): void {
    if (this.resetTimer) {
      clearTimeout(this.resetTimer)
      this.resetTimer = null
    }
    this.reset()
  }
}
