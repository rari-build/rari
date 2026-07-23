/// <reference path="../types.d.ts" />

import { primordials } from 'ext:core/mod.js'
import { lazyExtScript, nonEnumerableGetter } from 'ext:init_utilities/utilities.ts'

const lazyEventMod = lazyExtScript<DenoWebEventModule>('ext:deno_web/02_event.js')
const lazyGlobalInterfacesMod = lazyExtScript<DenoWebGlobalInterfacesModule>(
  'ext:deno_web/04_global_interfaces.js',
)

let eventTargetReady = false

export function ensureEventTargetReady(): void {
  if (eventTargetReady) return

  eventTargetReady = true
  const event = lazyEventMod()
  const { DedicatedWorkerGlobalScope } = lazyGlobalInterfacesMod()

  primordials.ObjectSetPrototypeOf(globalThis, DedicatedWorkerGlobalScope.prototype)
  event.saveGlobalThisReference(globalThis)
  event.setEventTargetData(globalThis)
}

export function lazyEvent(): DenoWebEventModule {
  return lazyEventMod()
}

export const lazyTimers = lazyExtScript<DenoWebTimersModule>('ext:deno_web/02_timers.js')
export const lazyAbortSignal = lazyExtScript<DenoWebAbortSignalModule>(
  'ext:deno_web/03_abort_signal.js',
)
export const lazyUrl = lazyExtScript<DenoWebUrlModule>('ext:deno_web/00_url.js')
export const lazyDomException = lazyExtScript<DenoWebDomExceptionModule>(
  'ext:deno_web/01_dom_exception.js',
)
export const lazyFile = lazyExtScript<DenoWebFileModule>('ext:deno_web/09_file.js')
export const lazyConsole = lazyExtScript<DenoWebConsoleModule>('ext:deno_web/01_console.js')

export const lazyBase64 = lazyExtScript<DenoWebBase64Module>('ext:deno_web/05_base64.js')
export const lazyEncoding = lazyExtScript<DenoWebEncodingModule>('ext:deno_web/08_text_encoding.js')
export const lazyStreams = lazyExtScript<DenoWebStreamsModule>('ext:deno_web/06_streams.js')
export const lazyCompression = lazyExtScript<DenoWebCompressionModule>(
  'ext:deno_web/14_compression.js',
)
export const lazyFileReader = lazyExtScript<DenoWebFileReaderModule>(
  'ext:deno_web/10_filereader.js',
)
export const lazyImageData = lazyExtScript<DenoWebImageDataModule>('ext:deno_web/16_image_data.js')
export const lazyMessagePort = lazyExtScript<DenoWebMessagePortModule>(
  'ext:deno_web/13_message_port.js',
)
export const lazyPerformance = lazyExtScript<DenoWebPerformanceModule>(
  'ext:deno_web/15_performance.js',
)

function lazyEventTargetMethod(
  method: 'addEventListener' | 'removeEventListener' | 'dispatchEvent',
): PropertyDescriptor {
  return {
    value(...args: readonly unknown[]) {
      ensureEventTargetReady()
      const { EventTarget } = lazyEventMod()
      const targetMethod = Reflect.get(EventTarget.prototype, method)
      if (typeof targetMethod !== 'function')
        throw new TypeError(`EventTarget.${method} is not a function`)

      // oxlint-disable-next-line typescript/no-unsafe-return -- EventTarget prototype method is dynamically resolved
      return Reflect.apply(targetMethod, globalThis, args)
    },
    writable: true,
    enumerable: true,
    configurable: true,
  }
}

export const lazyEventTargetMethods = {
  addEventListener: lazyEventTargetMethod('addEventListener'),
  removeEventListener: lazyEventTargetMethod('removeEventListener'),
  dispatchEvent: lazyEventTargetMethod('dispatchEvent'),
} satisfies PropertyDescriptorMap

/* oxlint-disable-next-line typescript/no-unnecessary-type-parameters, typescript/prefer-readonly-parameter-types */
function lazyLoadedEventProperty<V>(select: (mod: DenoWebEventModule) => V): PropertyDescriptor {
  return nonEnumerableGetter((): V => {
    ensureEventTargetReady()
    return select(lazyEventMod())
  })
}

export const lazyEventGlobalProps = {
  CloseEvent: lazyLoadedEventProperty(m => m.CloseEvent),
  CustomEvent: lazyLoadedEventProperty(m => m.CustomEvent),
  ErrorEvent: lazyLoadedEventProperty(m => m.ErrorEvent),
  Event: lazyLoadedEventProperty(m => m.Event),
  EventTarget: lazyLoadedEventProperty(m => m.EventTarget),
  MessageEvent: lazyLoadedEventProperty(m => m.MessageEvent),
  PromiseRejectionEvent: lazyLoadedEventProperty(m => m.PromiseRejectionEvent),
  ProgressEvent: lazyLoadedEventProperty(m => m.ProgressEvent),
  reportError: {
    get() {
      ensureEventTargetReady()
      return lazyEventMod().reportError
    },
    set() {},
    enumerable: true,
    configurable: true,
  },
} satisfies PropertyDescriptorMap
