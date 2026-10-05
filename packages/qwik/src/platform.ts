// rari as a Qwik Router platform.
//
// This module is bundled into the server entry that rari loads into V8. It is
// the equivalent of `@qwik.dev/router/middleware/node`: it turns the host's
// request descriptor into a `ServerRequestEvent`, hands it to Qwik Router's
// `requestHandler`, and streams the response back through the host's stream
// ops using the framing documented in `crates/rari/src/server/guest/stream.rs`:
//
// 1. first chunk: JSON header frame `{ status, headers: [[name, value], ...] }`
// 2. body bytes
// 3. `op_fizz_done(streamId)` exactly once, then the handler's promise resolves
import type { QwikManifest } from '@qwik.dev/core/optimizer'
import type {
  ClientConn,
  ServerRenderOptions,
  ServerRequestEvent,
} from '@qwik.dev/router/middleware/request-handler'
import { setServerPlatform } from '@qwik.dev/core/server'
import { mergeHeadersCookies, requestHandler } from '@qwik.dev/router/middleware/request-handler'

/** The request descriptor the rari host passes to `globalThis.__rariQwikHandle`. */
export interface RariHostRequest {
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
  /** The host's route decision (see {@link RariRoute}); absent when rari has no manifest. */
  readonly route?: RariRoute | null
}

/**
 * The route rari resolved for the request from `dist/server/routes.json`,
 * before Qwik ran. rari owns routing: unmatched URLs never reach Qwik unless
 * the app ships a `404` page, and the match is exposed on the request event as
 * `platform.rari.route` so server code can rely on the host's decision.
 */
export interface RariRoute {
  /** Matched route pattern (`/blog/[slug]`), or the request path for a 404 page. */
  readonly path: string
  /** Route params as the host parsed them (arrays for catch-all segments). */
  readonly params: Readonly<Record<string, string | readonly string[]>>
  /** Layout chain, outermost first, as paths relative to the routes dir. */
  readonly layouts: readonly string[]
  /** The host matched nothing and resolved the app's `404` page. */
  readonly notFound: boolean
}

/** `platform` on Qwik's request event when the app runs on rari. */
export interface RariPlatform {
  readonly ssr: true
  readonly rari: {
    readonly route: RariRoute | null
  }
}

export interface RariQwikHandlerOptions extends ServerRenderOptions {
  /** Client manifest, used to map QRL symbols to the served bundles. */
  readonly manifest?: QwikManifest
}

/** The handler installed on `globalThis.__rariQwikHandle`. */
export type RariQwikHandler = (request: RariHostRequest) => Promise<void>

type HostOp = (...args: readonly unknown[]) => unknown

interface HostStreamOps {
  readonly chunk: HostOp
  readonly chunkBytes: HostOp
  readonly done: HostOp
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function isHostOp(value: unknown): value is HostOp {
  return typeof value === 'function'
}

/** The rari runtime's stream ops, reached through the Deno core global. */
function hostOps(): HostStreamOps {
  const deno: unknown = Reflect.get(globalThis, 'Deno')
  const core: unknown = isRecord(deno) ? deno.core : undefined
  const ops: unknown = isRecord(core) ? core.ops : undefined
  if (
    isRecord(ops) &&
    isHostOp(ops.op_fizz_chunk) &&
    isHostOp(ops.op_fizz_chunk_bytes) &&
    isHostOp(ops.op_fizz_done)
  ) {
    return { chunk: ops.op_fizz_chunk, chunkBytes: ops.op_fizz_chunk_bytes, done: ops.op_fizz_done }
  }
  throw new Error('@rari/qwik: host stream ops are not available; is this running inside rari?')
}

function decodeBase64(value: string): Uint8Array<ArrayBuffer> {
  const binary = atob(value)
  const bytes = new Uint8Array(new ArrayBuffer(binary.length))
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i)
  return bytes
}

/** Flatten `Headers` into pairs, keeping each `Set-Cookie` separate. */
function headerPairs(headers: Headers): Array<[string, string]> {
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

/**
 * One response stream towards the host: header frame first, body bytes, then
 * exactly one `done`. Also the `getWritableStream` Qwik Router asks the
 * platform for.
 */
class HostResponseSink {
  headersSent = false
  private done = false

  constructor(
    private readonly ops: HostStreamOps,
    private readonly streamId: string,
  ) {}

  async sendHeaders(
    status: number,
    headers: ReadonlyArray<readonly [string, string]>,
  ): Promise<void> {
    this.headersSent = true
    await this.ops.chunk(this.streamId, JSON.stringify({ status, headers }))
  }

  async sendText(status: number, text: string): Promise<void> {
    await this.sendHeaders(status, TEXT_HEADERS)
    await this.ops.chunk(this.streamId, text)
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
    const headersWritten = this.sendHeaders(status, headerPairs(headers))
    const close = async () => {
      await headersWritten
      this.finish()
      resolve(null)
    }
    return new WritableStream<Uint8Array>({
      write: async chunk => {
        await headersWritten
        await this.ops.chunkBytes(this.streamId, chunk)
      },
      close,
      abort: close,
    })
  }
}

// The sink is the one mutable collaborator here by design.
// oxlint-disable typescript/prefer-readonly-parameter-types
function toServerRequestEvent(
  request: RariHostRequest,
  sink: HostResponseSink,
): ServerRequestEvent<null> {
  const method = request.method.toUpperCase()
  const body =
    method === 'GET' || method === 'HEAD' || request.bodyBase64 == null
      ? undefined
      : decodeBase64(request.bodyBase64)
  const init: RequestInit & { duplex?: 'half' } = {
    method,
    headers: request.headers.map(([name, value]) => [name, value]),
  }
  if (body) {
    init.body = body
    // Streaming-body requests need an explicit duplex mode in fetch impls.
    init.duplex = 'half'
  }
  const clientConn: ClientConn = { ip: request.clientIp ?? undefined }

  return {
    mode: 'server',
    url: new URL(request.url),
    locale: undefined,
    platform: { ssr: true, rari: { route: request.route ?? null } } satisfies RariPlatform,
    request: new Request(request.url, init),
    // The rari runtime injects the server's environment as `process.env`.
    env: { get: key => (typeof process === 'undefined' ? undefined : process.env[key]) },
    getClientConn: () => clientConn,
    getWritableStream: (status, headers, cookies, resolve) =>
      sink.writable(status, mergeHeadersCookies(headers, cookies), resolve),
  }
}
// oxlint-enable typescript/prefer-readonly-parameter-types

/**
 * Create the per-request handler: Qwik Router's `requestHandler` with rari as
 * the platform, streaming through the host's ops. Qwik's own option types
 * (render, manifest) are not deeply readonly.
 */
// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
export function createRariQwikHandler(options: Readonly<RariQwikHandlerOptions>): RariQwikHandler {
  const { manifest, ...renderOptions } = options
  // The platform maps QRL symbols to client bundles; it must be ready before
  // the first render. `setServerPlatform` is async, so await it per request.
  const platformReady = manifest ? setServerPlatform(manifest) : Promise.resolve()

  return async function handle(request: RariHostRequest): Promise<void> {
    await platformReady
    const sink = new HostResponseSink(hostOps(), request.streamId)

    try {
      const handled = await requestHandler(toServerRequestEvent(request, sink), renderOptions)
      if (!handled) {
        // Nothing matched and the router asked to fall through: rari has no
        // other handler for pages, so this is a 404.
        await sink.sendText(404, 'Not Found')
        return
      }

      // Redirects, rewrites and aborts complete with a message object, not an
      // Error; only genuine failures are worth a log line.
      const error = await handled.completion
      if (error instanceof Error) console.error('[rari/qwik] request failed:', error)
      if (!sink.headersSent) {
        // The handler finished without writing a response (nothing dirty).
        await sink.sendText(error ? 500 : 404, error ? 'Internal Server Error' : 'Not Found')
      }
    } catch (error) {
      if (!sink.headersSent) {
        await sink.sendText(500, 'Internal Server Error').catch(() => {
          // The stream is gone; nothing more to report to the client.
        })
      }
      throw error
    } finally {
      sink.finish()
    }
  }
}

declare global {
  // eslint-disable-next-line vars-on-top, no-var
  var __rariQwikHandle: RariQwikHandler | undefined
}

/** Install the handler the rari host calls per request. */
// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
export function installRariQwikHandler(options: Readonly<RariQwikHandlerOptions>): RariQwikHandler {
  const handler = createRariQwikHandler(options)
  globalThis.__rariQwikHandle = handler
  return handler
}
