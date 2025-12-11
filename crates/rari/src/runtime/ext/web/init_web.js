// oxlint-disable no-unused-vars
/* eslint-disable unused-imports/no-unused-imports */
import * as infra from 'ext:deno_web/00_infra.js'
import * as url from 'ext:deno_web/00_url.js'
import { DOMException } from 'ext:deno_web/01_dom_exception.js'
import * as mimesniff from 'ext:deno_web/01_mimesniff.js'
import * as event from 'ext:deno_web/02_event.js'
import * as structuredClone from 'ext:deno_web/02_structured_clone.js'
import * as timers from 'ext:deno_web/02_timers.js'
import * as abortSignal from 'ext:deno_web/03_abort_signal.js'
import * as globalInterfaces from 'ext:deno_web/04_global_interfaces.js'
import * as base64 from 'ext:deno_web/05_base64.js'
import * as streams from 'ext:deno_web/06_streams.js'
import * as encoding from 'ext:deno_web/08_text_encoding.js'
import * as file from 'ext:deno_web/09_file.js'
import * as fileReader from 'ext:deno_web/10_filereader.js'
import * as location from 'ext:deno_web/12_location.js'
import * as messagePort from 'ext:deno_web/13_message_port.js'
import * as compression from 'ext:deno_web/14_compression.js'
import * as performance from 'ext:deno_web/15_performance.js'
import * as imageData from 'ext:deno_web/16_image_data.js'
import * as errors from 'ext:init_web/init_errors.js'
import { applyToGlobal, nonEnumerable, writeable } from 'ext:rari/rari.js'

globalThis.Deno.refTimer = timers.refTimer
globalThis.Deno.unrefTimer = timers.unrefTimer

applyToGlobal({
  AbortController: nonEnumerable(abortSignal.AbortController),
  AbortSignal: nonEnumerable(abortSignal.AbortSignal),
  Blob: nonEnumerable(file.Blob),
  ByteLengthQueuingStrategy: nonEnumerable(
    streams.ByteLengthQueuingStrategy,
  ),
  CloseEvent: nonEnumerable(event.CloseEvent),
  CompressionStream: nonEnumerable(compression.CompressionStream),
  CountQueuingStrategy: nonEnumerable(
    streams.CountQueuingStrategy,
  ),
  CustomEvent: nonEnumerable(event.CustomEvent),
  DecompressionStream: nonEnumerable(compression.DecompressionStream),
  DOMException: nonEnumerable(DOMException),
  ErrorEvent: nonEnumerable(event.ErrorEvent),
  Event: nonEnumerable(event.Event),
  EventTarget: nonEnumerable(event.EventTarget),
  File: nonEnumerable(file.File),
  FileReader: nonEnumerable(fileReader.FileReader),
  MessageEvent: nonEnumerable(event.MessageEvent),
  Performance: nonEnumerable(performance.Performance),
  PerformanceEntry: nonEnumerable(performance.PerformanceEntry),
  PerformanceMark: nonEnumerable(performance.PerformanceMark),
  PerformanceMeasure: nonEnumerable(performance.PerformanceMeasure),
  PromiseRejectionEvent: nonEnumerable(event.PromiseRejectionEvent),
  ProgressEvent: nonEnumerable(event.ProgressEvent),
  ReadableStream: nonEnumerable(streams.ReadableStream),
  ReadableStreamDefaultReader: nonEnumerable(
    streams.ReadableStreamDefaultReader,
  ),
  TextDecoder: nonEnumerable(encoding.TextDecoder),
  TextEncoder: nonEnumerable(encoding.TextEncoder),
  TextDecoderStream: nonEnumerable(encoding.TextDecoderStream),
  TextEncoderStream: nonEnumerable(encoding.TextEncoderStream),
  TransformStream: nonEnumerable(streams.TransformStream),
  MessageChannel: nonEnumerable(messagePort.MessageChannel),
  MessagePort: nonEnumerable(messagePort.MessagePort),
  WritableStream: nonEnumerable(streams.WritableStream),
  WritableStreamDefaultWriter: nonEnumerable(
    streams.WritableStreamDefaultWriter,
  ),
  WritableStreamDefaultController: nonEnumerable(
    streams.WritableStreamDefaultController,
  ),
  ReadableByteStreamController: nonEnumerable(
    streams.ReadableByteStreamController,
  ),
  ReadableStreamBYOBReader: nonEnumerable(
    streams.ReadableStreamBYOBReader,
  ),
  ReadableStreamBYOBRequest: nonEnumerable(
    streams.ReadableStreamBYOBRequest,
  ),
  ReadableStreamDefaultController: nonEnumerable(
    streams.ReadableStreamDefaultController,
  ),
  TransformStreamDefaultController: nonEnumerable(
    streams.TransformStreamDefaultController,
  ),
  atob: writeable(base64.atob),
  btoa: writeable(base64.btoa),
  clearInterval: writeable(timers.clearInterval),
  clearTimeout: writeable(timers.clearTimeout),
  performance: writeable(performance.performance),
  reportError: writeable(event.reportError),
  refTimer: writeable(timers.refTimer),
  setImmediate: writeable(timers.setImmediate),
  setInterval: writeable(timers.setInterval),
  setTimeout: writeable(timers.setTimeout),
  unrefTimer: writeable(timers.unrefTimer),

  structuredClone: writeable(messagePort.structuredClone),
  ImageData: nonEnumerable(imageData.ImageData),
  URL: nonEnumerable(url.URL),
  URLSearchParams: nonEnumerable(url.URLSearchParams),
})
