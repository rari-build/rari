// @rari/core/guest: what a framework's server bundle needs to run as a rari guest.
//
// rari (the Rust host) calls one function per request inside V8 with a
// `GuestRequest` and expects the response back through its stream ops. This
// module is the framework-agnostic half of that exchange: the request types,
// the stream ops lookup, and a response sink that speaks the host's framing.
// Framework adapters (`@rari/qwik`, a Solid or Svelte adapter) build their
// platform shim on top of it, so every guest reports the same headers, honours
// the same route decision, and streams the same way.
//
// Framing, as documented in `crates/rari/src/server/guest/stream.rs`:
//
// 1. first chunk: JSON header frame `{ status, headers: [[name, value], ...] }`
// 2. body bytes
// 3. `op_fizz_done(streamId)` exactly once, then the handler's promise resolves
//
// No Node built-ins: this is bundled into the server entry rari evaluates.

/**
 * The route rari resolved for a request from `dist/server/routes.json`,
 * before the framework ran. rari owns routing: unmatched URLs never reach the
 * guest unless the app ships a not-found page, and the decision is handed to
 * the framework so server code can rely on it.
 */
export interface HostRoute {
  /** Matched route pattern (`/blog/[slug]`), or the request path for a not-found page. */
  readonly path: string
  /** Route params as the host parsed them (arrays for catch-all segments). */
  readonly params: Readonly<Record<string, string | readonly string[]>>
  /**
   * Layout chain, outermost first, as paths relative to the routes dir;
   * resolved with rari's route-file grammar (`page@name`, `page!`, `layout-name`, `layout!`).
   */
  readonly layouts: readonly string[]
  /** The host matched nothing and resolved the app's not-found page. */
  readonly notFound: boolean
}

/** The request descriptor the rari host passes to a guest's handler. */
export interface GuestRequest {
  /** Stream id for the host's chunk ops. */
  readonly streamId: string
  /** Absolute request URL. */
  readonly url: string
  readonly method: string
  /** Request headers as name/value pairs (repeated names appear repeatedly). */
  readonly headers: ReadonlyArray<readonly [string, string]>
  /** Base64-encoded body for methods that carry one. */
  readonly bodyBase64?: string | null
  readonly clientIp?: string | null
  /** The host's route decision (see {@link HostRoute}); absent when rari has no manifest. */
  readonly route?: HostRoute | null
}

/** A guest's per-request handler, installed on a global the host calls. */
export type GuestHandler = (request: GuestRequest) => Promise<void>

type HostOp = (...args: readonly unknown[]) => unknown
type HostTryOp = (...args: readonly unknown[]) => number

/**
 * The rari runtime's stream ops. The `*Try` ops are synchronous and return
 * `0` sent, `1` channel full (fall back to the async op), `2` disconnected;
 * a chunk sent through them costs no event-loop turn.
 */
export interface HostStreamOps {
  readonly chunk: HostOp
  readonly chunkBytes: HostOp
  readonly done: HostOp
  readonly chunkTry?: HostTryOp
  readonly chunkBytesTry?: HostTryOp
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function isHostOp(value: unknown): value is HostOp {
  return typeof value === 'function'
}

function isHostTryOp(value: unknown): value is HostTryOp {
  return typeof value === 'function'
}

/** The rari runtime's stream ops, reached through the Deno core global. */
export function hostStreamOps(): HostStreamOps {
  const deno: unknown = Reflect.get(globalThis, 'Deno')
  const core: unknown = isRecord(deno) ? deno.core : undefined
  const ops: unknown = isRecord(core) ? core.ops : undefined
  if (
    isRecord(ops) &&
    isHostOp(ops.op_fizz_chunk) &&
    isHostOp(ops.op_fizz_chunk_bytes) &&
    isHostOp(ops.op_fizz_done)
  ) {
    return {
      chunk: ops.op_fizz_chunk,
      chunkBytes: ops.op_fizz_chunk_bytes,
      done: ops.op_fizz_done,
      chunkTry: isHostTryOp(ops.op_fizz_chunk_try) ? ops.op_fizz_chunk_try : undefined,
      chunkBytesTry: isHostTryOp(ops.op_fizz_chunk_bytes_try)
        ? ops.op_fizz_chunk_bytes_try
        : undefined,
    }
  }
  throw new Error(
    '@rari/core/guest: host stream ops are not available; is this running inside rari?',
  )
}

/** Decode a {@link GuestRequest.bodyBase64} body. */
export function decodeRequestBody(value: string): Uint8Array<ArrayBuffer> {
  const binary = atob(value)
  const bytes = new Uint8Array(new ArrayBuffer(binary.length))
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i)
  return bytes
}

/**
 * A `Request` for the guest's framework from the host's descriptor; bodies
 * only for methods that carry one.
 */
export function toRequest(request: GuestRequest): Request {
  const method = request.method.toUpperCase()
  const body =
    method === 'GET' || method === 'HEAD' || request.bodyBase64 == null
      ? undefined
      : decodeRequestBody(request.bodyBase64)
  const init: RequestInit & { duplex?: 'half' } = {
    method,
    headers: request.headers.map(([name, value]) => [name, value]),
  }
  if (body) {
    init.body = body
    // Streaming-body requests need an explicit duplex mode in fetch impls.
    init.duplex = 'half'
  }
  return new Request(request.url, init)
}

/** Flatten `Headers` into pairs, keeping each `Set-Cookie` separate. */
export function headerPairs(headers: Headers): Array<[string, string]> {
  const pairs: Array<[string, string]> = []
  const setCookies = typeof headers.getSetCookie === 'function' ? headers.getSetCookie() : undefined
  headers.forEach((value, name) => {
    if (setCookies && name.toLowerCase() === 'set-cookie') return
    pairs.push([name, value])
  })
  for (const cookie of setCookies ?? []) pairs.push(['set-cookie', cookie])
  return pairs
}

const TEXT_HEADERS: ReadonlyArray<readonly [string, string]> = [
  ['content-type', 'text/plain; charset=utf-8'],
]

/** Thrown when the host dropped the response (the client went away). */
export class HostDisconnectedError extends Error {
  constructor() {
    super('rari host: stream receiver disconnected')
    this.name = 'HostDisconnectedError'
  }
}

/**
 * One response stream towards the host: header frame first, body bytes, then
 * exactly one `done`. {@link HostResponseSink.writable} is the shape most
 * frameworks' request handlers ask a platform for.
 *
 * Chunks go through the host's synchronous try-send ops whenever the channel
 * has room, so a render that never hits backpressure completes in a single
 * event-loop turn; only a full channel makes a write await the async op, and
 * later writes queue behind it to keep the byte order.
 */
export class HostResponseSink {
  headersSent = false
  private done = false
  /** The async send in flight under backpressure; later chunks chain on it. */
  private pending: Promise<void> | null = null

  constructor(
    private readonly ops: HostStreamOps,
    private readonly streamId: string,
  ) {}

  /** Send one chunk, synchronously when the host allows it. */
  private push(
    tryOp: HostTryOp | undefined,
    asyncOp: HostOp,
    payload: string | Uint8Array,
  ): Promise<void> | undefined {
    if (this.pending === null && tryOp) {
      const status = tryOp(this.streamId, payload)
      if (status === 0) return undefined
      if (status === 2) throw new HostDisconnectedError()
    }
    const send = async () => {
      await asyncOp(this.streamId, payload)
    }
    const chained = this.pending === null ? send() : this.pending.then(send)
    this.pending = chained
    void chained.then(
      () => {
        if (this.pending === chained) this.pending = null
      },
      () => {
        if (this.pending === chained) this.pending = null
      },
    )
    return chained
  }

  sendHeaders(
    status: number,
    headers: ReadonlyArray<readonly [string, string]>,
  ): Promise<void> | undefined {
    this.headersSent = true
    return this.push(this.ops.chunkTry, this.ops.chunk, JSON.stringify({ status, headers }))
  }

  async sendText(status: number, text: string): Promise<void> {
    await this.sendHeaders(status, TEXT_HEADERS)
    await this.push(this.ops.chunkTry, this.ops.chunk, text)
  }

  /** Send a whole `Response` (status, headers, streamed body). */
  async sendResponse(response: Response): Promise<void> {
    await this.sendHeaders(response.status, headerPairs(response.headers))
    if (response.body) {
      for await (const chunk of response.body) {
        await this.push(this.ops.chunkBytesTry, this.ops.chunkBytes, chunk)
      }
    }
  }

  finish(): void {
    if (this.done) return
    this.done = true
    this.ops.done(this.streamId)
  }

  writable(
    status: number,
    headers: Headers,
    resolve: (response: null) => void,
  ): WritableStream<Uint8Array> {
    void this.sendHeaders(status, headerPairs(headers))
    const close = async () => {
      if (this.pending) await this.pending
      this.finish()
      resolve(null)
    }
    return new WritableStream<Uint8Array>({
      write: (chunk): Promise<void> | undefined =>
        this.push(this.ops.chunkBytesTry, this.ops.chunkBytes, chunk),
      close,
      abort: close,
    })
  }
}
