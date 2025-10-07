export interface HMRError {
  message: string
  stack?: string
  filePath?: string
  timestamp: number
}

export class HMRErrorOverlay {
  private overlay: HTMLDivElement | null = null
  private currentError: HMRError | null = null

  show(error: HMRError): void {
    this.currentError = error

    if (this.overlay) {
      this.updateOverlay(error)
    }
    else {
      this.createOverlay(error)
    }
  }

  hide(): void {
    if (this.overlay) {
      this.overlay.remove()
      this.overlay = null
      this.currentError = null
    }
  }

  isVisible(): boolean {
    return this.overlay !== null
  }

  getCurrentError(): HMRError | null {
    return this.currentError
  }

  private createOverlay(error: HMRError): void {
    this.overlay = document.createElement('div')
    this.overlay.id = 'rari-hmr-error-overlay'
    this.styleOverlay(this.overlay)
    this.updateOverlay(error)
    document.body.appendChild(this.overlay)
  }

  private updateOverlay(error: HMRError): void {
    if (!this.overlay)
      return

    const fileInfo = error.filePath
      ? `<div style="margin-bottom: 1rem; padding: 0.75rem; background: rgba(0, 0, 0, 0.2); border-radius: 0.375rem; font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace; font-size: 0.875rem;">
          <strong>File:</strong> ${this.escapeHtml(error.filePath)}
        </div>`
      : ''

    const stackTrace = error.stack
      ? `<details style="margin-top: 1rem; cursor: pointer;">
          <summary style="font-weight: 600; margin-bottom: 0.5rem; user-select: none;">Stack Trace</summary>
          <pre style="margin: 0; padding: 0.75rem; background: rgba(0, 0, 0, 0.2); border-radius: 0.375rem; overflow-x: auto; font-size: 0.875rem; line-height: 1.5; white-space: pre-wrap; word-break: break-word;">${this.escapeHtml(error.stack)}</pre>
        </details>`
      : ''

    this.overlay.innerHTML = `
      <div style="position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0, 0, 0, 0.85); z-index: 999999; display: flex; align-items: center; justify-content: center; padding: 2rem; backdrop-filter: blur(4px);">
        <div style="background: #1e1e1e; color: #e0e0e0; border-radius: 0.5rem; padding: 2rem; max-width: 50rem; width: 100%; max-height: 90vh; overflow-y: auto; box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 10px 10px -5px rgba(0, 0, 0, 0.4); border: 1px solid #ef4444;">
          <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 1.5rem;">
            <div style="display: flex; align-items: center; gap: 0.75rem;">
              <svg style="width: 2rem; height: 2rem; color: #ef4444;" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
              </svg>
              <h1 style="margin: 0; font-size: 1.5rem; font-weight: 700; color: #ef4444;">Build Error</h1>
            </div>
            <button onclick="document.getElementById('rari-hmr-error-overlay').remove()" style="background: transparent; border: none; color: #9ca3af; cursor: pointer; padding: 0.5rem; border-radius: 0.25rem; transition: all 0.2s; font-size: 1.5rem; line-height: 1; width: 2rem; height: 2rem; display: flex; align-items: center; justify-content: center;" onmouseover="this.style.background='rgba(255,255,255,0.1)'; this.style.color='#e0e0e0'" onmouseout="this.style.background='transparent'; this.style.color='#9ca3af'">×</button>
          </div>

          ${fileInfo}

          <div style="margin-bottom: 1.5rem;">
            <h2 style="margin: 0 0 0.75rem 0; font-size: 1rem; font-weight: 600; color: #fca5a5;">Error Message:</h2>
            <pre style="margin: 0; padding: 1rem; background: rgba(239, 68, 68, 0.1); border-left: 4px solid #ef4444; border-radius: 0.375rem; overflow-x: auto; font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace; font-size: 0.875rem; line-height: 1.5; white-space: pre-wrap; word-break: break-word; color: #fca5a5;">${this.escapeHtml(error.message)}</pre>
          </div>

          ${stackTrace}

          <div style="margin-top: 1.5rem; padding-top: 1.5rem; border-top: 1px solid #374151; display: flex; gap: 0.75rem; align-items: center;">
            <button onclick="window.location.reload()" style="padding: 0.625rem 1.25rem; background: #ef4444; color: white; border: none; border-radius: 0.375rem; cursor: pointer; font-weight: 600; font-size: 0.875rem; transition: all 0.2s;" onmouseover="this.style.background='#dc2626'" onmouseout="this.style.background='#ef4444'">Reload Page</button>
            <button onclick="document.getElementById('rari-hmr-error-overlay').remove()" style="padding: 0.625rem 1.25rem; background: #374151; color: #e0e0e0; border: none; border-radius: 0.375rem; cursor: pointer; font-weight: 600; font-size: 0.875rem; transition: all 0.2s;" onmouseover="this.style.background='#4b5563'" onmouseout="this.style.background='#374151'">Dismiss</button>
            <span style="margin-left: auto; font-size: 0.75rem; color: #9ca3af;">
              ${new Date(error.timestamp).toLocaleTimeString()}
            </span>
          </div>
        </div>
      </div>
    `
  }

  private styleOverlay(element: HTMLDivElement): void {
    element.style.position = 'fixed'
    element.style.zIndex = '999999'
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div')
    div.textContent = text
    return div.innerHTML
  }
}

let overlayInstance: HMRErrorOverlay | null = null

export function getErrorOverlay(): HMRErrorOverlay {
  if (!overlayInstance) {
    overlayInstance = new HMRErrorOverlay()
  }
  return overlayInstance
}
