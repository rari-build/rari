/// <reference path="../core/types.d.ts" />

import { lazyExtModule } from 'ext:init_utilities/utilities.ts'

interface ReactDefaultExport {
  createElement?: (
    component: unknown,
    props: unknown,
    ...children: unknown[]
  ) => unknown
  Fragment?: symbol
  Suspense?: symbol
  use?: <T>(usable: T | Promise<T>) => T
  cache?: <T extends (...args: unknown[]) => unknown>(fn: T) => T
}

interface ReactVendorNamespace extends ReactDefaultExport {
  default?: ReactDefaultExport
}

interface ReactDomServerNamespace {
  renderToReadableStream: (
    element: unknown,
    options?: { onError?: (error: unknown) => void },
  ) => Promise<ReadableStream>
}

interface FlightClientNamespace {
  createFromReadableStream: (
    stream: ReadableStream,
    options?: { ssrManifest?: unknown },
  ) => Promise<unknown>
}

interface FlightServerNamespace {
  renderToReadableStream: (
    element: unknown,
    bundlerConfig: unknown,
    options?: { onError?: (error: unknown) => void },
  ) => Promise<ReadableStream>
  decodeAction?: (
    body: FormData,
    serverManifest: Record<string, { id: string, chunks: string[] }>,
  ) => Promise<(() => Promise<unknown>) | null>
  decodeReply?: (
    body: string | FormData,
    serverManifest: Record<string, { id: string, chunks: string[] }>,
  ) => Promise<unknown>
}

const lazyReact = lazyExtModule<ReactVendorNamespace>('ext:rari/react/vendor/react.js')
const lazyReactDomServer = lazyExtModule<ReactDomServerNamespace>(
  'ext:rari/react/vendor/react-dom-server.js',
)
const lazyFlightClient = lazyExtModule<FlightClientNamespace>(
  'ext:rari/react/vendor/react-server-dom-webpack-client.js',
)
const lazyFlightServer = lazyExtModule<FlightServerNamespace>(
  'ext:rari/react/vendor/react-server-dom-webpack-server.js',
)

function installReactGlobal(react: ReactVendorNamespace): void {
  if (!g.React?.createElement) {
    const resolved = react.default?.createElement ? react.default : react
    g.React = resolved as typeof g.React
  }
}

export function loadFullReactVendors(): boolean {
  try {
    const react = lazyReact()
    const reactDomServer = lazyReactDomServer()
    const flightClient = lazyFlightClient()
    const flightServer = lazyFlightServer()

    installReactGlobal(react)
    g['~reactServer'] = reactDomServer
    g['~flightClient'] = flightClient
    g['~reactServerRenderer'] = flightServer

    return !!(
      typeof g.React?.createElement === 'function'
      && typeof g['~reactServer']?.renderToReadableStream === 'function'
      && typeof g['~flightClient']?.createFromReadableStream === 'function'
      && typeof g['~reactServerRenderer']?.renderToReadableStream === 'function'
    )
  }
  catch (e) {
    console.warn('[rari] Could not load React server modules:', (e as Error)?.message ?? e)
    return false
  }
}

export function loadRscReactVendors(): boolean {
  try {
    const react = lazyReact()
    const flightServer = lazyFlightServer()

    installReactGlobal(react)
    g['~reactServerRenderer'] = flightServer

    return !!(
      typeof g.React?.createElement === 'function'
      && typeof g['~reactServerRenderer']?.renderToReadableStream === 'function'
      && typeof g['~reactServerRenderer']?.decodeAction === 'function'
      && typeof g['~reactServerRenderer']?.decodeReply === 'function'
    )
  }
  catch (e) {
    console.warn('[rari] Could not load RSC React modules:', (e as Error)?.message ?? e)
    return false
  }
}
