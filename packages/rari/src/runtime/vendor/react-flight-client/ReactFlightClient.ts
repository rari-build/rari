/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * Vendored from: https://github.com/facebook/react
 * Original file: packages/react-client/src/ReactFlightClient.js
 * Modifications: Stripped DEV code, converted to TypeScript, adapted for rari
 */

import type * as React from 'react'

import type {
  ClientReference,
  ClientReferenceMetadata,
  ServerConsumerModuleMap,
  StringDecoder,
} from './ReactFlightClientConfig'

import {
  createStringDecoder,
  preloadModule,
  readFinalStringChunk,
  readPartialStringChunk,
  requireModule,
  resolveClientReference,
} from './ReactFlightClientConfig'

export interface Thenable<T> extends Promise<T> {
  status?: 'pending' | 'fulfilled' | 'rejected'
  value?: T
  reason?: any
}

const REACT_ELEMENT_TYPE = Symbol.for('react.transitional.element') || Symbol.for('react.element')
const REACT_LAZY_TYPE = Symbol.for('react.lazy')

export type CallServerCallback = <A, T>(id: string, args: A) => Promise<T>

const ROW_ID = 0
const ROW_TAG = 1
const ROW_CHUNK_BY_NEWLINE = 3

type RowParserState = 0 | 1 | 2 | 3 | 4

const PENDING = 'pending'
const BLOCKED = 'blocked'
const RESOLVED_MODEL = 'resolved_model'
const RESOLVED_MODULE = 'resolved_module'
const INITIALIZED = 'fulfilled'
const ERRORED = 'rejected'

const __PROTO__ = '__proto__'

interface PendingChunk<T> {
  status: 'pending'
  value: null | Array<(value: T) => void>
  reason: null | Array<(error: any) => void>
  then: (resolve: (value: T) => void, reject?: (error: any) => void) => void
}

interface BlockedChunk<T> {
  status: 'blocked'
  value: null | Array<(value: T) => void>
  reason: null | Array<(error: any) => void>
  then: (resolve: (value: T) => void, reject?: (error: any) => void) => void
}

interface ResolvedModelChunk<T> {
  status: 'resolved_model'
  value: string
  reason: Response
  then: (resolve: (value: T) => void, reject?: (error: any) => void) => void
}

interface ResolvedModuleChunk<T> {
  status: 'resolved_module'
  value: ClientReference<T>
  reason: null
  then: (resolve: (value: T) => void, reject?: (error: any) => void) => void
}

interface InitializedChunk<T> {
  status: 'fulfilled'
  value: T
  reason: null
  then: (resolve: (value: T) => void, reject?: (error: any) => void) => void
}

interface ErroredChunk<T> {
  status: 'rejected'
  value: null
  reason: any
  then: (resolve: (value: T) => void, reject?: (error: any) => void) => void
}

type SomeChunk<T>
  = | PendingChunk<T>
    | BlockedChunk<T>
    | ResolvedModelChunk<T>
    | ResolvedModuleChunk<T>
    | InitializedChunk<T>
    | ErroredChunk<T>

function ReactPromise(this: any, status: string, value: any, reason: any) {
  this.status = status
  this.value = value
  this.reason = reason
}

ReactPromise.prototype = Object.create(Promise.prototype) as any

ReactPromise.prototype.then = function <T>(
  this: SomeChunk<T>,
  resolve: (value: T) => void,
  reject?: (error: any) => void,
) {
  const chunk: SomeChunk<T> = this

  switch (chunk.status) {
    case RESOLVED_MODEL:
      initializeModelChunk(chunk)
      break
    case RESOLVED_MODULE:
      initializeModuleChunk(chunk)
      break
  }

  switch (chunk.status) {
    case INITIALIZED:
      if (resolve) {
        resolve(chunk.value)
      }
      break
    case PENDING:
    case BLOCKED:
      if (resolve) {
        if (chunk.value === null) {
          chunk.value = [] as any
        }
        if (!Array.isArray(chunk.value)) {
          resolve(chunk.value as any)
          break
        }
        chunk.value.push(resolve as any)
      }
      if (reject) {
        if (chunk.reason === null) {
          chunk.reason = [] as any
        }
        if (!Array.isArray(chunk.reason)) {
          break
        }
        chunk.reason.push(reject as any)
      }
      break
    default:
      if (reject) {
        reject(chunk.reason)
      }
      break
  }
}

export interface Response {
  _bundlerConfig: ServerConsumerModuleMap
  _callServer: CallServerCallback
  _chunks: Map<number, SomeChunk<any>>
  _stringDecoder: StringDecoder
  _closed: boolean
  _closedReason: any
}

export interface StreamState {
  _rowState: RowParserState
  _rowID: number
  _rowTag: number
  _rowLength: number
  _buffer: Array<Uint8Array>
}

function readChunk<T>(chunk: SomeChunk<T>): T {
  switch (chunk.status) {
    case RESOLVED_MODEL:
      initializeModelChunk(chunk)
      break
    case RESOLVED_MODULE:
      initializeModuleChunk(chunk)
      break
  }

  switch (chunk.status) {
    case INITIALIZED:
      return chunk.value
    case PENDING:
    case BLOCKED:
      // eslint-disable-next-line no-throw-literal
      throw chunk as any
    default:
      throw new Error(String(chunk.reason))
  }
}

export function getRoot<T>(response: Response): Thenable<T> {
  const chunk = getChunk(response, 0)
  return chunk as any
}

function createPendingChunk<T>(): PendingChunk<T> {
  return new (ReactPromise as any)(PENDING, null, null)
}

function createBlockedChunk<T>(): BlockedChunk<T> {
  return new (ReactPromise as any)(BLOCKED, null, null)
}

function createErrorChunk<T>(error: any): ErroredChunk<T> {
  return new (ReactPromise as any)(ERRORED, null, error)
}

function wakeChunk<T>(
  listeners: Array<(value: T) => void>,
  value: T,
): void {
  for (let i = 0; i < listeners.length; i++) {
    const listener = listeners[i]
    if (typeof listener === 'function') {
      listener(value)
    }
  }
}

function wakeChunkIfInitialized<T>(
  chunk: SomeChunk<T>,
  resolveListeners: Array<(value: T) => void>,
  rejectListeners: null | Array<(error: any) => void>,
): void {
  switch (chunk.status) {
    case INITIALIZED:
      wakeChunk(resolveListeners, chunk.value)
      break
    case PENDING:
    case BLOCKED:
      if (chunk.value !== null && !Array.isArray(chunk.value)) {
        chunk.value = resolveListeners as any
        break
      }
      if (chunk.value) {
        for (let i = 0; i < resolveListeners.length; i++) {
          chunk.value.push(resolveListeners[i])
        }
      }
      else {
        chunk.value = resolveListeners as any
      }

      if (chunk.reason !== null && !Array.isArray(chunk.reason)) {
        chunk.reason = rejectListeners as any
        break
      }
      if (chunk.reason) {
        if (rejectListeners) {
          for (let i = 0; i < rejectListeners.length; i++) {
            chunk.reason.push(rejectListeners[i])
          }
        }
      }
      else {
        chunk.reason = rejectListeners as any
      }
      break
    case ERRORED:
      if (rejectListeners) {
        wakeChunk(rejectListeners, chunk.reason)
      }
      break
  }
}

function triggerErrorOnChunk<T>(
  chunk: SomeChunk<T>,
  error: any,
): void {
  if (chunk.status !== PENDING && chunk.status !== BLOCKED) {
    return
  }

  const listeners = chunk.reason
  const erroredChunk: ErroredChunk<T> = chunk as any
  erroredChunk.status = ERRORED
  erroredChunk.reason = error

  if (listeners !== null) {
    for (let i = 0; i < listeners.length; i++) {
      listeners[i](error)
    }
  }
}

function createResolvedModelChunk<T>(
  response: Response,
  value: string,
): ResolvedModelChunk<T> {
  return new (ReactPromise as any)(RESOLVED_MODEL, value, response)
}

function createResolvedModuleChunk<T>(
  value: ClientReference<T>,
): ResolvedModuleChunk<T> {
  return new (ReactPromise as any)(RESOLVED_MODULE, value, null)
}

function createInitializedTextChunk(
  value: string,
): InitializedChunk<string> {
  return new (ReactPromise as any)(INITIALIZED, value, null)
}

function resolveModelChunk<T>(
  response: Response,
  chunk: SomeChunk<T>,
  value: string,
): void {
  if (chunk.status !== PENDING && chunk.status !== BLOCKED) {
    return
  }

  const resolveListeners = Array.isArray(chunk.value) ? chunk.value : null
  const rejectListeners = Array.isArray(chunk.reason) ? chunk.reason : null
  const resolvedChunk: ResolvedModelChunk<T> = chunk as any
  resolvedChunk.status = RESOLVED_MODEL
  resolvedChunk.value = value
  resolvedChunk.reason = response

  initializeModelChunk(resolvedChunk)
  if (resolveListeners !== null) {
    wakeChunkIfInitialized(chunk, resolveListeners, rejectListeners)
  }
}

function resolveModuleChunk<T>(
  response: Response,
  chunk: SomeChunk<T>,
  value: ClientReference<T>,
): void {
  if (chunk.status !== PENDING && chunk.status !== BLOCKED && chunk.status !== ERRORED) {
    return
  }

  const resolveListeners = Array.isArray(chunk.value) ? chunk.value : null
  const rejectListeners = Array.isArray(chunk.reason) ? chunk.reason : null
  const resolvedChunk: ResolvedModuleChunk<T> = chunk as any
  resolvedChunk.status = RESOLVED_MODULE
  resolvedChunk.value = value
  resolvedChunk.reason = null

  initializeModuleChunk(resolvedChunk)
  if (resolveListeners !== null) {
    wakeChunkIfInitialized(chunk, resolveListeners, rejectListeners)
  }
}

let initializingChunk: SomeChunk<any> | null = null
let initializingHandler: InitializationHandler | null = null

interface InitializationHandler {
  parent: null | InitializationHandler
  chunk: null | BlockedChunk<any>
  value: any
  deps: number
  errored: boolean
  reason: any
}

function initializeModelChunk<T>(chunk: ResolvedModelChunk<T>): void {
  const prevChunk = initializingChunk
  const prevHandler = initializingHandler
  initializingChunk = null
  initializingHandler = null

  const resolvedModel = chunk.value
  const response = chunk.reason

  const cyclicChunk: BlockedChunk<T> = chunk as any
  cyclicChunk.status = BLOCKED
  cyclicChunk.value = null
  cyclicChunk.reason = null

  initializingChunk = cyclicChunk

  try {
    const value: T = parseModel(response, resolvedModel)

    if (initializingHandler !== null) {
      const handler: InitializationHandler = initializingHandler
      if (handler.errored) {
        throw handler.reason
      }
      if (handler.deps > 0) {
        handler.value = value
        handler.chunk = cyclicChunk
        return
      }
    }

    const initializedChunk: InitializedChunk<T> = chunk as any
    initializedChunk.status = INITIALIZED
    initializedChunk.value = value
    initializedChunk.reason = null
  }
  catch (error) {
    const erroredChunk: ErroredChunk<T> = chunk as any
    erroredChunk.status = ERRORED
    erroredChunk.reason = error
  }
  finally {
    initializingChunk = prevChunk
    initializingHandler = prevHandler
  }
}

function initializeModuleChunk<T>(chunk: ResolvedModuleChunk<T>): void {
  try {
    const value: T = requireModule(chunk.value)
    const initializedChunk: InitializedChunk<T> = chunk as any
    initializedChunk.status = INITIALIZED
    initializedChunk.value = value
    initializedChunk.reason = null
  }
  catch (error) {
    const erroredChunk: ErroredChunk<T> = chunk as any
    erroredChunk.status = ERRORED
    erroredChunk.reason = error
  }
}

export function reportGlobalError(response: Response, error: Error): void {
  response._closed = true
  response._closedReason = error
  response._chunks.forEach((chunk) => {
    if (chunk.status === PENDING) {
      triggerErrorOnChunk(chunk, error)
    }
  })
}

function getChunk(response: Response, id: number): SomeChunk<any> {
  const chunks = response._chunks
  let chunk = chunks.get(id)

  if (!chunk) {
    if (response._closed) {
      chunk = createErrorChunk(response._closedReason)
    }
    else {
      chunk = createPendingChunk()
    }
    chunks.set(id, chunk)
  }

  return chunk
}

function createElement(
  response: Response,
  type: any,
  key: any,
  props: any,
): React.ReactElement {
  props ??= {}

  const element: any = {
    $$typeof: REACT_ELEMENT_TYPE,
    type,
    key,
    ref: null,
    props,
  }

  if (initializingHandler !== null) {
    const handler = initializingHandler
    initializingHandler = handler.parent

    if (handler.errored) {
      const erroredChunk: ErroredChunk<any> = createErrorChunk(handler.reason)
      return createLazyChunkWrapper(erroredChunk) as any
    }
    if (handler.deps > 0) {
      const blockedChunk: BlockedChunk<any> = createBlockedChunk()
      handler.value = element
      handler.chunk = blockedChunk
      return createLazyChunkWrapper(blockedChunk) as any
    }
  }

  return element
}

function createLazyChunkWrapper<T>(
  chunk: SomeChunk<T>,
): React.LazyExoticComponent<any> {
  const lazyType: any = {
    $$typeof: REACT_LAZY_TYPE,
    _payload: chunk,
    _init: readChunk,
  }

  return lazyType
}

function getOutlinedModel<T>(
  response: Response,
  reference: string,
  parentObject: any,
  key: string,
  map: (model: any) => T,
): T {
  const path = reference.split(':')
  const id = Number.parseInt(path[0], 16)
  const chunk = getChunk(response, id)

  switch (chunk.status) {
    case RESOLVED_MODEL:
      initializeModelChunk(chunk)
      break
    case RESOLVED_MODULE:
      initializeModuleChunk(chunk)
      break
  }

  switch (chunk.status) {
    case INITIALIZED: {
      let value = chunk.value
      for (let i = 1; i < path.length; i++) {
        value = value[path[i]]
      }

      return map(value)
    }
    case PENDING:
    case BLOCKED: {
      return createLazyChunkWrapper(chunk) as any
    }
    default:
      if (initializingHandler) {
        initializingHandler.errored = true
        initializingHandler.reason = chunk.reason
      }
      else {
        initializingHandler = {
          parent: null,
          chunk: null,
          value: null,
          deps: 0,
          errored: true,
          reason: chunk.reason,
        }
      }

      return null as any
  }
}

function createModel(model: any): any {
  return model
}

function parseModelString(
  response: Response,
  parentObject: any,
  key: string,
  value: string,
): any {
  if (value[0] === '$') {
    if (value === '$') {
      if (initializingHandler !== null && key === '0') {
        initializingHandler = {
          parent: initializingHandler,
          chunk: null,
          value: null,
          deps: 0,
          errored: false,
          reason: null,
        }
      }

      return REACT_ELEMENT_TYPE
    }

    switch (value[1]) {
      case '$': {
        return value.slice(1)
      }
      case 'L': {
        const id = Number.parseInt(value.slice(2), 16)
        const chunk = getChunk(response, id)
        return createLazyChunkWrapper(chunk)
      }
      case '@': {
        const id = Number.parseInt(value.slice(2), 16)
        const chunk = getChunk(response, id)
        return chunk
      }
      case 'S': {
        return Symbol.for(value.slice(2))
      }
      case 'F': {
        const ref = value.slice(2)
        return getOutlinedModel(response, ref, parentObject, key, model => loadServerReference(response, model))
      }
      case 'Q': {
        const ref = value.slice(2)
        return getOutlinedModel(response, ref, parentObject, key, createMap)
      }
      case 'W': {
        const ref = value.slice(2)
        return getOutlinedModel(response, ref, parentObject, key, createSet)
      }
      case 'I': {
        return Infinity
      }
      case '-': {
        if (value === '$-0') {
          return -0
        }
        else {
          return -Infinity
        }
      }
      case 'N': {
        return Number.NaN
      }
      case 'u': {
        return undefined
      }
      case 'D': {
        return new Date(Date.parse(value.slice(2)))
      }
      case 'n': {
        return BigInt(value.slice(2))
      }
      default: {
        const ref = value.slice(1)
        return getOutlinedModel(response, ref, parentObject, key, createModel)
      }
    }
  }

  return value
}

function parseModelTuple(
  response: Response,
  value: Array<any>,
): any {
  const tuple = value

  if (tuple[0] === REACT_ELEMENT_TYPE) {
    return createElement(response, tuple[1], tuple[2], tuple[3])
  }

  return value
}

function reviveModel(
  response: Response,
  value: any,
  parentObject: any,
  key: string,
): any {
  if (typeof value === 'string') {
    if (value[0] === '$') {
      return parseModelString(response, parentObject, key, value)
    }

    return value
  }

  if (typeof value !== 'object' || value === null) {
    return value
  }

  if (Array.isArray(value)) {
    for (let i = 0; i < value.length; i++) {
      value[i] = reviveModel(response, value[i], value, `${i}`)
    }

    if (value[0] === REACT_ELEMENT_TYPE) {
      return parseModelTuple(response, value)
    }

    return value
  }

  for (const k in value) {
    if (k === __PROTO__) {
      delete value[k]
    }
    else {
      const walked = reviveModel(response, value[k], value, k)
      if (walked !== undefined) {
        value[k] = walked
      }
      else {
        delete value[k]
      }
    }
  }

  return value
}

function parseModel<T>(response: Response, json: string): T {
  const rawModel = JSON.parse(json)
  return reviveModel(response, rawModel, { '': rawModel }, '')
}

function createMap(model: Array<[any, any]>): Map<any, any> {
  return new Map(model)
}

function createSet(model: Array<any>): Set<any> {
  return new Set(model)
}

function loadServerReference<A, T>(
  response: Response,
  metaData: { id: string, bound: null | any },
): (...args: A[]) => Promise<T> {
  const callServer = response._callServer
  const id = metaData.id

  return function (...args: A[]): Promise<T> {
    return callServer(id, args)
  }
}

function missingCall(): never {
  throw new Error(
    'Trying to call a function from "use server" but the callServer option '
    + 'was not implemented in your router runtime.',
  )
}

function ResponseInstance(
  this: any,
  bundlerConfig: ServerConsumerModuleMap,
  callServer: void | CallServerCallback,
) {
  const chunks: Map<number, SomeChunk<any>> = new Map()
  this._bundlerConfig = bundlerConfig
  this._callServer = callServer !== undefined ? callServer : missingCall
  this._chunks = chunks
  this._stringDecoder = createStringDecoder()
  this._closed = false
  this._closedReason = null
}

export function createResponse(
  bundlerConfig: ServerConsumerModuleMap,
  callServer: void | CallServerCallback,
): Response {
  return new (ResponseInstance as any)(
    bundlerConfig,
    callServer,
  )
}

export function createStreamState(): StreamState {
  return {
    _rowState: ROW_ID,
    _rowID: 0,
    _rowTag: 0,
    _rowLength: 0,
    _buffer: [],
  }
}

function resolveModel(
  response: Response,
  id: number,
  model: string,
): void {
  const chunks = response._chunks
  const chunk = chunks.get(id)

  if (!chunk) {
    chunks.set(id, createResolvedModelChunk(response, model))
  }
  else {
    resolveModelChunk(response, chunk, model)
  }
}

function resolveText(
  response: Response,
  id: number,
  text: string,
): void {
  const chunks = response._chunks
  chunks.set(id, createInitializedTextChunk(text))
}

function resolveModule(
  response: Response,
  id: number,
  model: string,
): void {
  const chunks = response._chunks
  const chunk = chunks.get(id)
  const clientReferenceMetadata: ClientReferenceMetadata = parseModel(response, model)
  const clientReference = resolveClientReference<any>(
    clientReferenceMetadata,
  )

  const promise = preloadModule(clientReference)

  if (promise) {
    let blockedChunk: BlockedChunk<any>
    if (!chunk) {
      blockedChunk = createBlockedChunk()
      chunks.set(id, blockedChunk)
    }
    else {
      blockedChunk = chunk as any
      blockedChunk.status = BLOCKED
    }

    promise.then(
      () => resolveModuleChunk(response, blockedChunk, clientReference),
      error => triggerErrorOnChunk(blockedChunk, error),
    )
  }
  else {
    if (!chunk) {
      chunks.set(id, createResolvedModuleChunk(clientReference))
    }
    else {
      resolveModuleChunk(response, chunk, clientReference)
    }
  }
}

function resolveErrorModel(
  response: Response,
  id: number,
  digest: string,
): void {
  const error = new Error(
    'An error occurred in the Server Components render. The specific message is omitted in production'
    + ' builds to avoid leaking sensitive details. A digest property is included on this error instance which'
    + ' may provide additional details about the nature of the error.',
  )
  ;(error as any).digest = digest

  const chunks = response._chunks
  const chunk = chunks.get(id)

  if (!chunk) {
    chunks.set(id, createErrorChunk(error))
  }
  else {
    triggerErrorOnChunk(chunk, error)
  }
}

function processFullStringRow(
  response: Response,
  id: number,
  tag: number,
  row: string,
): void {
  switch (tag) {
    case 73: /* "I" */ {
      resolveModule(response, id, row)
      return
    }
    case 69: /* "E" */ {
      const errorInfo = JSON.parse(row)
      resolveErrorModel(response, id, errorInfo.digest)
      return
    }
    case 84: /* "T" */ {
      resolveText(response, id, row)
      return
    }
    default: {
      resolveModel(response, id, row)
    }
  }
}

export function processBinaryChunk(
  response: Response,
  streamState: StreamState,
  chunk: Uint8Array,
): void {
  let i = 0
  let rowState = streamState._rowState
  let rowID = streamState._rowID
  let rowTag = streamState._rowTag
  let rowLength = streamState._rowLength
  const buffer = streamState._buffer
  const chunkLength = chunk.length

  while (i < chunkLength) {
    let lastIdx = -1

    switch (rowState) {
      case ROW_ID: {
        const byte = chunk[i++]
        if (byte === 58 /* ":" */) {
          rowState = ROW_TAG
        }
        else {
          rowID = (rowID << 4) | (byte > 96 ? byte - 87 : byte - 48)
        }
        continue
      }
      case ROW_TAG: {
        const resolvedRowTag = chunk[i]
        if (
          resolvedRowTag === 84
          || /* "T" */ (resolvedRowTag > 64 && resolvedRowTag < 91) /* "A"-"Z" */
        ) {
          rowTag = resolvedRowTag
          rowState = ROW_CHUNK_BY_NEWLINE
          i++
        }
        else {
          rowTag = 0
          rowState = ROW_CHUNK_BY_NEWLINE
        }
        continue
      }
      case ROW_CHUNK_BY_NEWLINE: {
        lastIdx = chunk.indexOf(10 /* "\n" */, i)
        break
      }
    }

    const offset = chunk.byteOffset + i

    if (lastIdx > -1) {
      const length = lastIdx - i
      const lastChunk = new Uint8Array(chunk.buffer, offset, length)

      const stringDecoder = response._stringDecoder
      let row = ''
      for (let j = 0; j < buffer.length; j++) {
        row += readPartialStringChunk(stringDecoder, buffer[j])
      }
      row += readFinalStringChunk(stringDecoder, lastChunk)

      processFullStringRow(response, rowID, rowTag, row)

      i = lastIdx
      if (rowState === ROW_CHUNK_BY_NEWLINE) {
        i++
      }

      rowState = ROW_ID
      rowTag = 0
      rowID = 0
      rowLength = 0
      buffer.length = 0
    }
    else {
      const length = chunk.byteLength - i
      const remainingSlice = new Uint8Array(chunk.buffer, offset, length)
      buffer.push(remainingSlice)
      rowLength -= remainingSlice.byteLength
      break
    }
  }

  streamState._rowState = rowState
  streamState._rowID = rowID
  streamState._rowTag = rowTag
  streamState._rowLength = rowLength
}

export function processStringChunk(
  response: Response,
  streamState: StreamState,
  chunk: string,
): void {
  let i = 0
  let rowState = streamState._rowState
  let rowID = streamState._rowID
  let rowTag = streamState._rowTag
  const chunkLength = chunk.length

  while (i < chunkLength) {
    let lastIdx = -1

    switch (rowState) {
      case ROW_ID: {
        const byte = chunk.charCodeAt(i++)
        if (byte === 58 /* ":" */) {
          rowState = ROW_TAG
        }
        else {
          rowID = (rowID << 4) | (byte > 96 ? byte - 87 : byte - 48)
        }
        continue
      }
      case ROW_TAG: {
        const resolvedRowTag = chunk.charCodeAt(i)
        if (
          resolvedRowTag === 84
          || /* "T" */ (resolvedRowTag > 64 && resolvedRowTag < 91) /* "A"-"Z" */
        ) {
          rowTag = resolvedRowTag
          rowState = ROW_CHUNK_BY_NEWLINE
          i++
        }
        else {
          rowTag = 0
          rowState = ROW_CHUNK_BY_NEWLINE
        }
        continue
      }
      case ROW_CHUNK_BY_NEWLINE: {
        lastIdx = chunk.indexOf('\n', i)
        break
      }
    }

    if (lastIdx > -1) {
      const lastChunk = chunk.slice(i, lastIdx)
      processFullStringRow(response, rowID, rowTag, lastChunk)

      i = lastIdx
      if (rowState === ROW_CHUNK_BY_NEWLINE) {
        i++
      }

      rowState = ROW_ID
      rowTag = 0
      rowID = 0
    }
    else {
      break
    }
  }

  streamState._rowState = rowState
  streamState._rowID = rowID
  streamState._rowTag = rowTag
}

export function close(response: Response): void {
  response._closed = true
  response._chunks.forEach((chunk) => {
    if (chunk.status === PENDING) {
      triggerErrorOnChunk(chunk, new Error('Connection closed.'))
    }
  })
}
