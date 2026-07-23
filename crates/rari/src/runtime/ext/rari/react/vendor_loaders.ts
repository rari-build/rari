/// <reference path="../core/types.d.ts" />

import { lazyExtModule } from 'ext:init_utilities/utilities.ts'

interface ReactDefaultExport {
  readonly createElement?: (
    component: unknown,
    props: unknown,
    ...children: readonly unknown[]
  ) => unknown
  readonly Fragment?: symbol
  readonly Suspense?: symbol
  readonly use?: <T>(usable: T | Promise<T>) => T
  readonly cache?: <T extends (...args: readonly unknown[]) => unknown>(fn: T) => T
}

interface ReactVendorNamespace extends ReactDefaultExport {
  readonly default?: Readonly<ReactDefaultExport>
}

interface ReactDomServerNamespace {
  renderToReadableStream: (
    element: unknown,
    options?: Readonly<{ onError?: (error: unknown) => void }>,
  ) => Promise<ReadableStream>
}

interface FlightClientNamespace {
  createFromReadableStream: (
    stream: ReadableStream,
    options?: Readonly<{ ssrManifest?: unknown }>,
  ) => Promise<unknown>
}

interface FlightServerNamespace {
  renderToReadableStream: (
    element: unknown,
    bundlerConfig: unknown,
    options?: Readonly<{ onError?: (error: unknown) => void }>,
  ) => Promise<ReadableStream>
  decodeAction?: (
    body: FormData,
    serverManifest: Readonly<{
      readonly [key: string]: Readonly<{ readonly id: string; readonly chunks: readonly string[] }>
    }>,
  ) => Promise<(() => Promise<unknown>) | null>
  decodeReply?: (
    body: string | FormData,
    serverManifest: Readonly<{
      readonly [key: string]: Readonly<{ readonly id: string; readonly chunks: readonly string[] }>
    }>,
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
    g.React = resolved as NonNullable<typeof g.React> // oxlint-disable-line typescript/no-unsafe-type-assertion -- React vendor namespace merge
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

    return (
      typeof g.React?.createElement === 'function' &&
      typeof g['~reactServer']?.renderToReadableStream === 'function' &&
      typeof g['~flightClient']?.createFromReadableStream === 'function' &&
      typeof g['~reactServerRenderer']?.renderToReadableStream === 'function'
    )
  } catch (e: unknown) {
    const message = e instanceof Error ? e.message : String(e)
    console.warn('[rari] Could not load React server modules:', message)
    return false
  }
}

export function loadRscReactVendors(): boolean {
  try {
    const react = lazyReact()
    const flightServer = lazyFlightServer()

    installReactGlobal(react)
    g['~reactServerRenderer'] = flightServer

    return (
      typeof g.React?.createElement === 'function' &&
      typeof g['~reactServerRenderer']?.renderToReadableStream === 'function'
    )
  } catch (e: unknown) {
    const message = e instanceof Error ? e.message : String(e)
    console.warn('[rari] Could not load RSC React modules:', message)
    return false
  }
}
