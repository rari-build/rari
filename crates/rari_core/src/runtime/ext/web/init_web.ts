/// <reference path="../types.d.ts" />

import {
  applyToDeno,
  applyToGlobal,
  propNonEnumerableLazyLoaded,
  propWritableLazyLoaded,
} from 'ext:init_utilities/utilities.ts'
import {
  lazyAbortSignal,
  lazyBase64,
  lazyCompression,
  lazyDomException,
  lazyEncoding,
  lazyEventGlobalProps,
  lazyEventTargetMethods,
  lazyFile,
  lazyFileReader,
  lazyImageData,
  lazyMessagePort,
  lazyPerformance,
  lazyStreams,
  lazyTimers,
  lazyUrl,
} from './shared_loaders.ts'
import './init_errors.ts'

applyToDeno({
  refTimer: propWritableLazyLoaded(t => t.refTimer, lazyTimers),
  unrefTimer: propWritableLazyLoaded(t => t.unrefTimer, lazyTimers),
})

applyToGlobal({
  ...lazyEventTargetMethods,
  ...lazyEventGlobalProps,
  AbortController: propNonEnumerableLazyLoaded(m => m.AbortController, lazyAbortSignal),
  AbortSignal: propNonEnumerableLazyLoaded(m => m.AbortSignal, lazyAbortSignal),
  Blob: propNonEnumerableLazyLoaded(m => m.Blob, lazyFile),
  ByteLengthQueuingStrategy: propNonEnumerableLazyLoaded(
    s => s.ByteLengthQueuingStrategy,
    lazyStreams,
  ),
  CompressionStream: propNonEnumerableLazyLoaded(
    c => c.CompressionStream,
    lazyCompression,
  ),
  CountQueuingStrategy: propNonEnumerableLazyLoaded(
    s => s.CountQueuingStrategy,
    lazyStreams,
  ),
  DecompressionStream: propNonEnumerableLazyLoaded(
    c => c.DecompressionStream,
    lazyCompression,
  ),
  DOMException: propNonEnumerableLazyLoaded(m => m.DOMException, lazyDomException),
  File: propNonEnumerableLazyLoaded(m => m.File, lazyFile),
  FileReader: propNonEnumerableLazyLoaded(
    m => m.FileReader,
    lazyFileReader,
  ),
  Performance: propNonEnumerableLazyLoaded(m => m.Performance, lazyPerformance),
  PerformanceEntry: propNonEnumerableLazyLoaded(m => m.PerformanceEntry, lazyPerformance),
  PerformanceMark: propNonEnumerableLazyLoaded(m => m.PerformanceMark, lazyPerformance),
  PerformanceMeasure: propNonEnumerableLazyLoaded(m => m.PerformanceMeasure, lazyPerformance),
  ReadableStream: propNonEnumerableLazyLoaded(
    s => s.ReadableStream,
    lazyStreams,
  ),
  ReadableStreamDefaultReader: propNonEnumerableLazyLoaded(
    s => s.ReadableStreamDefaultReader,
    lazyStreams,
  ),
  TextDecoder: propNonEnumerableLazyLoaded(
    m => m.TextDecoder,
    lazyEncoding,
  ),
  TextEncoder: propNonEnumerableLazyLoaded(
    m => m.TextEncoder,
    lazyEncoding,
  ),
  TextDecoderStream: propNonEnumerableLazyLoaded(
    m => m.TextDecoderStream,
    lazyEncoding,
  ),
  TextEncoderStream: propNonEnumerableLazyLoaded(
    m => m.TextEncoderStream,
    lazyEncoding,
  ),
  TransformStream: propNonEnumerableLazyLoaded(
    s => s.TransformStream,
    lazyStreams,
  ),
  MessageChannel: propNonEnumerableLazyLoaded(m => m.MessageChannel, lazyMessagePort),
  MessagePort: propNonEnumerableLazyLoaded(m => m.MessagePort, lazyMessagePort),
  WritableStream: propNonEnumerableLazyLoaded(
    s => s.WritableStream,
    lazyStreams,
  ),
  WritableStreamDefaultWriter: propNonEnumerableLazyLoaded(
    s => s.WritableStreamDefaultWriter,
    lazyStreams,
  ),
  WritableStreamDefaultController: propNonEnumerableLazyLoaded(
    s => s.WritableStreamDefaultController,
    lazyStreams,
  ),
  ReadableByteStreamController: propNonEnumerableLazyLoaded(
    s => s.ReadableByteStreamController,
    lazyStreams,
  ),
  ReadableStreamBYOBReader: propNonEnumerableLazyLoaded(
    s => s.ReadableStreamBYOBReader,
    lazyStreams,
  ),
  ReadableStreamBYOBRequest: propNonEnumerableLazyLoaded(
    s => s.ReadableStreamBYOBRequest,
    lazyStreams,
  ),
  ReadableStreamDefaultController: propNonEnumerableLazyLoaded(
    s => s.ReadableStreamDefaultController,
    lazyStreams,
  ),
  TransformStreamDefaultController: propNonEnumerableLazyLoaded(
    s => s.TransformStreamDefaultController,
    lazyStreams,
  ),
  atob: propWritableLazyLoaded(m => m.atob, lazyBase64),
  btoa: propWritableLazyLoaded(m => m.btoa, lazyBase64),
  clearInterval: propWritableLazyLoaded(t => t.clearInterval, lazyTimers),
  clearTimeout: propWritableLazyLoaded(t => t.clearTimeout, lazyTimers),
  performance: {
    get() {
      return lazyPerformance().performance
    },
    set() {},
    enumerable: true,
    configurable: true,
  },
  refTimer: propWritableLazyLoaded(t => t.refTimer, lazyTimers),
  setImmediate: propWritableLazyLoaded(t => t.setImmediate, lazyTimers),
  setInterval: propWritableLazyLoaded(t => t.setInterval, lazyTimers),
  setTimeout: propWritableLazyLoaded(t => t.setTimeout, lazyTimers),
  unrefTimer: propWritableLazyLoaded(t => t.unrefTimer, lazyTimers),
  structuredClone: propWritableLazyLoaded(m => m.structuredClone, lazyMessagePort),
  ImageData: propNonEnumerableLazyLoaded(
    m => m.ImageData,
    lazyImageData,
  ),
  URL: propNonEnumerableLazyLoaded(m => m.URL, lazyUrl),
  URLSearchParams: propNonEnumerableLazyLoaded(m => m.URLSearchParams, lazyUrl),
})
