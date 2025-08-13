/// <reference types="vite/client" />

declare module 'virtual:rsc-integration' {
  export const rscClient: {
    configure: (config: {
      enableStreaming?: boolean
      maxRetries?: number
      retryDelay?: number
      timeout?: number
      [key: string]: any
    }) => void
  }
}

declare module 'virtual:rsc-client-components'
