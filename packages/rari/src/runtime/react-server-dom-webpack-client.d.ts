declare module 'react-server-dom-webpack/client' {
  export interface Options {
    callServer?: <A, T>(id: string, args: A) => Promise<T>
    moduleMap?: any
    moduleLoading?: any
  }

  export interface Thenable<T> extends Promise<T> {
    status?: 'pending' | 'fulfilled' | 'rejected'
    value?: T
    reason?: any
  }

  export function createFromReadableStream<T>(
    stream: ReadableStream<Uint8Array>,
    options?: Options,
  ): Thenable<T>

  export function createFromFetch<T>(
    promiseForResponse: Promise<Response>,
    options?: Options,
  ): Thenable<T>
}
