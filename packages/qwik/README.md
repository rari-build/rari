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

## What stays Qwik, what rari takes over

The app is a standard Qwik Router project; nothing in `src/` is rari-specific. Compared with a Node/Cloudflare adapter:

| Area | Standard Qwik 2 | On rari |
| --- | --- | --- |
| Scaffold | `pnpm create qwik@rc` | same, then `pnpm add rari @rari/qwik` and use `rariQwik()` in `vite.config.ts` |
| Dev server, HMR, devtools | `vite --mode ssr` | same (Qwik's dev server; rari is not in the loop in dev) |
| Routes, layouts, `404.tsx`, `error.tsx`, `index@layout`, `layout-name`, `index!`, `.md`/`.mdx` pages, `plugin@*.ts` | Qwik Router conventions | same files; rari's scanner reads the same conventions for its manifest |
| Loaders, actions, `server$`, cookies, redirects, CSRF, `<head>` | Qwik Router | Qwik Router (`requestHandler`), unchanged |
| Build | `vite build` + adapter config in `adapters/` | `vite build` (or `rari build`); no adapter directory, no `entry.<platform>.tsx` |
| Serve | `node server/entry.node-server.js` | `rari start` |
| URL → route | Qwik's trie, per request | rari's router first; unmatched URLs are host 404s (`x-rari-route: miss`), the match is on `platform.rari.route` |
| Page cache | none (or a CDN via `cacheControl()`) | rari's response cache; the page's own `cacheControl()` sets the TTL, `no-store`/`private`/`max-age=0` opt out |
| Static files, compression, ETag/304 | the adapter's | rari's |
| SSG (`adapters/static`) | supported | not applicable; rari caches rendered pages instead |

Two differences to know about:

- `platform.rari.route` exists only when rari serves the request (production); in `vite dev` it is `null`. For params and the current URL prefer Qwik's `useLocation()`, which is identical in both.
- `vite preview` uses Qwik's Node middleware (`src/entry.preview.tsx`), not rari. Use `rari start` to preview the production host.

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
