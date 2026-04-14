/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * Vendored from: https://github.com/facebook/react
 * Original file: packages/react-server-dom-webpack/src/client/ReactFlightDOMClientBrowser.js
 * Modifications: Public API for rari
 */

import type { CallServerCallback, Response, Thenable } from './ReactFlightClient'
import {
  close,
  createResponse,
  createStreamState,
  getRoot,
  processBinaryChunk,
  reportGlobalError,
} from './ReactFlightClient'

export interface Options {
  callServer?: CallServerCallback
  moduleMap?: any
  moduleLoading?: any
}

export function createFromReadableStream<T>(
  stream: ReadableStream<Uint8Array>,
  options?: Options,
): Thenable<T> {
  const bundlerConfig = options?.moduleMap ? { moduleMap: options.moduleMap, moduleLoading: options.moduleLoading } : {}

  const response = createResponse(
    bundlerConfig,
    options?.callServer,
  )

  startReadingFromStream(response, stream, () => {
    close(response)
  })

  return getRoot(response)
}

export function createFromFetch<T>(
  promiseForResponse: Promise<globalThis.Response>,
  options?: Options,
): Thenable<T> {
  const bundlerConfig = options?.moduleMap ? { moduleMap: options.moduleMap, moduleLoading: options.moduleLoading } : {}

  const response = createResponse(
    bundlerConfig,
    options?.callServer,
  )

  promiseForResponse.then(
    (r) => {
      if (!r.body) {
        reportGlobalError(response, new Error('Response has no body'))
        return
      }
      startReadingFromStream(response, r.body, () => {
        close(response)
      })
    },
    (e) => {
      if (isAbortError(e)) {
        close(response)
        return
      }
      reportGlobalError(response, e)
    },
  )

  const root = getRoot(response)
  return root as Thenable<T>
}

function startReadingFromStream(
  response: Response,
  stream: ReadableStream<Uint8Array>,
  onDone: () => void,
): void {
  const streamState = createStreamState()
  const reader = stream.getReader()

  function progress(result: ReadableStreamReadResult<Uint8Array>): void | Promise<void> {
    if (result.done) {
      return onDone()
    }

    const buffer: Uint8Array = result.value
    processBinaryChunk(response, streamState, buffer)
    return reader.read().then(progress).catch(error)
  }

  function error(e: any) {
    if (isAbortError(e)) {
      close(response)
      return
    }
    reportGlobalError(response, e)
  }

  reader.read().then(progress).catch(error)
}

function isAbortError(e: any): boolean {
  return e instanceof Error && (e.name === 'AbortError' || e.message === 'The operation was aborted.' || e.message === 'The user aborted a request.')
}
