// @rari/qwik: the Qwik adapter for rari.
//
// rari is the host (Rust HTTP server, router, cache, V8 runtime pool); Qwik is
// the guest. This package builds a Qwik Router app into:
//
// - the normal Qwik client output (`dist/client`), which rari serves statically,
// - a self-contained server bundle (`dist/server/qwik-server-entry.mjs`) that
//   wraps the app's `entry.ssr` in Qwik Router's own `requestHandler`, with
//   rari as the platform (see `./platform`),
// - rari's route manifest (`dist/server/routes.json`) so the Rust router can
//   match routes, key its cache and answer 404s without knowing Qwik.
//
// Loaders, actions, `server$`, cookies, redirects, and rendering are Qwik's;
// the request/response transport and everything around it is rari's.
export { buildQwik, buildQwikServer } from './build'
export type { BuildQwikOptions, BuildQwikResult, BuildQwikServerOptions } from './build'
export type { RariHostRequest, RariPlatform, RariRoute } from './platform'
export { rariQwik } from './vite'
export type { RariQwikOptions } from './vite'
