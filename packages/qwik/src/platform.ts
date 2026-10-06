// rari as a Qwik Router platform.
//
// This module is bundled into the server entry that rari loads into V8. It is
// the equivalent of `@qwik.dev/router/middleware/node`: it turns the host's
// `GuestRequest` into a `ServerRequestEvent`, hands it to Qwik Router's
// `requestHandler`, and streams the response back through `@rari/core/guest`'s
// response sink. Everything about the host (request shape, route decision,
// stream framing) comes from `@rari/core/guest`; only the Qwik half is here.
import type { QwikManifest } from '@qwik.dev/core/optimizer'
import type {
  ClientConn,
  ServerRenderOptions,
  ServerRequestEvent,
} from '@qwik.dev/router/middleware/request-handler'
import type { GuestHandler, GuestRequest, HostRoute } from '@rari/core/guest'
import { setServerPlatform } from '@qwik.dev/core/server'
import { mergeHeadersCookies, requestHandler } from '@qwik.dev/router/middleware/request-handler'
import { HostResponseSink, hostStreamOps, toRequest } from '@rari/core/guest'

export type { GuestRequest, HostRoute } from '@rari/core/guest'

/**
 * `platform` on Qwik's request event when the app runs on rari. `rari.route`
 * is the host's route decision (`null` outside rari, e.g. in `vite dev`).
 */
export interface RariPlatform {
  readonly ssr: true
  readonly rari: {
    readonly route: HostRoute | null
  }
}

export interface RariQwikHandlerOptions extends ServerRenderOptions {
  /** Client manifest, used to map QRL symbols to the served bundles. */
  readonly manifest?: QwikManifest
}

// The sink is the one mutable collaborator here by design.
// oxlint-disable typescript/prefer-readonly-parameter-types
function toServerRequestEvent(
  request: GuestRequest,
  sink: HostResponseSink,
): ServerRequestEvent<null> {
  const clientConn: ClientConn = { ip: request.clientIp ?? undefined }

  return {
    mode: 'server',
    url: new URL(request.url),
    locale: undefined,
    platform: { ssr: true, rari: { route: request.route ?? null } } satisfies RariPlatform,
    request: toRequest(request),
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
export function createRariQwikHandler(options: Readonly<RariQwikHandlerOptions>): GuestHandler {
  const { manifest, ...renderOptions } = options
  // The platform maps QRL symbols to client bundles; it must be ready before
  // the first render. `setServerPlatform` is async, so await it per request.
  const platformReady = manifest ? setServerPlatform(manifest) : Promise.resolve()

  return async function handle(request: GuestRequest): Promise<void> {
    await platformReady
    const sink = new HostResponseSink(hostStreamOps(), request.streamId)

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
  var __rariQwikHandle: GuestHandler | undefined
}

/** Install the handler the rari host calls per request. */
// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
export function installRariQwikHandler(options: Readonly<RariQwikHandlerOptions>): GuestHandler {
  const handler = createRariQwikHandler(options)
  globalThis.__rariQwikHandle = handler
  return handler
}
