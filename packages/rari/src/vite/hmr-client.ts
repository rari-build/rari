/// <reference path="./hmr-types.d.ts" />

import { getErrorOverlay } from './hmr-error-overlay'

interface HMRErrorEvent {
  msg: string
  stack?: string
  file?: string
  t: number
  count?: number
  max?: number
}

interface HMRErrorClearedEvent {
  t: number
}

export function initializeHMRClient(): void {
  if (typeof window === 'undefined' || !import.meta.hot) {
    return
  }

  const overlay = getErrorOverlay()

  import.meta.hot.on('rari:hmr-error', (data: HMRErrorEvent) => {
    console.error('[HMR] Build error:', data.msg)

    if (data.file) {
      console.error('[HMR] File:', data.file)
    }

    if (data.stack) {
      console.error('[HMR] Stack:', data.stack)
    }

    overlay.show({
      message: data.msg,
      stack: data.stack,
      filePath: data.file,
      timestamp: data.t,
    })

    if (data.count && data.max) {
      if (data.count >= data.max) {
        console.error(
          `[HMR] Maximum error count (${data.max}) reached. `
          + 'Consider restarting the dev server if issues persist.',
        )
      }
      else if (data.count >= data.max - 2) {
        console.warn(
          `[HMR] Error count: ${data.count}/${data.max}. `
          + 'Approaching maximum error threshold.',
        )
      }
    }
  })

  import.meta.hot.on('rari:hmr-error-cleared', (_data: HMRErrorClearedEvent) => {
    overlay.hide()
  })

  import.meta.hot.on('rari:server-component-updated', () => {
    if (overlay.isVisible()) {
      overlay.hide()
    }
  })

  import.meta.hot.on('vite:error', (data: any) => {
    console.error('[HMR] Vite error:', data)

    overlay.show({
      message: data.err?.message || 'Unknown Vite error',
      stack: data.err?.stack,
      filePath: data.err?.file,
      timestamp: Date.now(),
    })
  })
}

if (typeof window !== 'undefined') {
  initializeHMRClient()
}
