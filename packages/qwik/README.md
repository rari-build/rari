# @rari/qwik

Qwik adapter for [rari](https://rari.build): run a [Qwik Router](https://qwik.dev) app on rari's Rust host.

rari is the host — HTTP server, routing, response cache, static assets, and a pool of V8 runtimes. Qwik is the guest: this package builds your app into a self-contained server bundle that rari loads into V8 and streams per request. Loaders, actions, `server$`, cookies, redirects, `<head>` and rendering are Qwik Router's own `requestHandler`; rari is just another platform for it, like the Node or Cloudflare adapters.

## Setup

```bash
pnpm add rari @rari/qwik @qwik.dev/core@rc @qwik.dev/router@rc
```

```ts
// vite.config.ts
import { rariQwik } from '@rari/qwik/vite'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [rariQwik()],
})
```

Your app is a normal Qwik Router app: `src/root.tsx`, `src/entry.ssr.tsx` (`createRenderer`), and `src/routes/**`.

- `vite dev` — Qwik's own dev server, exactly as in any Qwik project.
- `vite build` (or `rari build`) — the Qwik client build into `dist/client`, then the rari server bundle, route manifest and server config into `dist/server`.
- `rari start` — serves the app from the Rust host. `dist/server/config.json` records `"framework": "qwik"`, so no flag or environment variable is needed (`--framework qwik` / `RARI_FRAMEWORK=qwik` override it).

## Options

```ts
rariQwik({
  srcDir: 'src', // Qwik source dir
  routesDir: 'src/routes', // Qwik Router routes
  entrySsr: 'src/entry.ssr', // the createRenderer() entry
  experimental: ['pendingBoundary'], // Qwik optimizer flags, e.g. <Pending> out-of-order streaming
})
```

`buildQwik()` / `buildQwikServer()` from `@rari/qwik/build` run the same steps programmatically.

## How a request flows

rari owns routing. Where rari has a feature Qwik also has, rari's wins; Qwik does what makes Qwik Qwik: resumable rendering, loaders, actions, `server$`.

1. rari serves `/build/*`, `/assets/*` and other files from `dist/client` directly.
2. rari matches the URL against `dist/server/routes.json` (emitted from your `src/routes` by `@rari/core`'s scanner, Qwik's trailing slash and `q-loader-*.json` data requests normalised away). No match and no `404` page in the app means a host 404 (`x-rari-route: miss`) and the runtime is never touched.
3. Anonymous page GETs are answered from the host's response cache (precompressed, ETag/304, cookie-partitioned).
4. On a miss the bundle's handler runs on a pooled V8 runtime with the request (URL, method, headers, body, client IP) and the host's route decision. `@rari/qwik/platform` builds a `ServerRequestEvent` with `platform.rari.route` set and hands it to Qwik Router's `requestHandler`.
5. Qwik writes status and headers, then HTML chunks, which the host streams out as they are produced; every response carries `x-rari-route` with the matched pattern. Cacheable pages are stored for the next hit.

Form actions (`POST ?qaction=…`) and `server$` RPC endpoints are Qwik's; the host routes every method on a matched page path to the handler.

```ts
export const useHostRoute = routeLoader$(({ platform }) => (platform as RariPlatform).rari.route)
```
