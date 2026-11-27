// Copyright 2018-2026 the Deno authors. MIT license.

const { core, primordials } = __bootstrap
const { op_fs_events_open, op_fs_events_poll } = core.ops
const { BadResourcePrototype, InterruptedPrototype } = core
const {
  ArrayIsArray,
  ObjectPrototypeIsPrototypeOf,
  PromiseResolve,
  SymbolAsyncIterator,
  SymbolDispose,
} = primordials

interface WatchFsOptions {
  readonly recursive?: boolean
  readonly ignore?: string | readonly string[]
}

class FsWatcher {
  readonly #rid: number
  #promise: Promise<FsEvent | null> | undefined
  #closed = false

  constructor(paths: readonly string[], options: WatchFsOptions) {
    const { recursive = true, ignore } = options
    const ignorePaths = ignore === undefined ? [] : ArrayIsArray(ignore) ? ignore : [ignore]
    this.#rid = op_fs_events_open(recursive, ignorePaths, paths)
  }

  unref(): void {
    if (this.#promise) core.unrefOpPromise(this.#promise)
  }

  ref(): void {
    if (this.#promise) core.refOpPromise(this.#promise)
  }

  async next(): Promise<IteratorResult<FsEvent>> {
    if (this.#closed) {
      return { value: undefined, done: true }
    }
    try {
      this.#promise = op_fs_events_poll(this.#rid)
      const value = await this.#promise
      return value ? { value, done: false } : { value: undefined, done: true }
    } catch (error) {
      if (typeof error === 'object' && error !== null) {
        if (ObjectPrototypeIsPrototypeOf(BadResourcePrototype, error)) {
          this.#closed = true
          return { value: undefined, done: true }
        }
        if (ObjectPrototypeIsPrototypeOf(InterruptedPrototype, error)) {
          return { value: undefined, done: true }
        }
      }
      throw error
    }
  }

  async return(value?: unknown): Promise<IteratorResult<FsEvent>> {
    this.#close()
    return PromiseResolve({ value, done: true })
  }

  close(): void {
    this.#close()
  }

  [SymbolAsyncIterator](): this {
    return this
  }

  [SymbolDispose](): void {
    this.#close()
  }

  #close(): void {
    if (!this.#closed) {
      this.#closed = true
      core.tryClose(this.#rid)
    }
  }
}

function watchFs(
  paths: string | readonly string[],
  options: WatchFsOptions = { recursive: true },
): FsWatcher {
  return new FsWatcher(ArrayIsArray(paths) ? paths : [paths], options)
}

export { FsWatcher, watchFs }
