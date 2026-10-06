// @rari/core: the framework-agnostic contracts shared by rari's host and its
// framework adapters (@rari/react, @rari/qwik, and others).
//
// rari is the host: a Rust HTTP server, router and cache around a V8 runtime
// pool. A framework is a guest renderer loaded into that runtime. This package
// holds what every adapter needs regardless of framework: the route manifest
// schema the Rust router reads from `dist/server/routes.json`, the file-system
// route scanner that produces it (parametrised by each framework's file
// conventions and rari's route-file grammar), the metadata-route conventions,
// the guest-side request/response contract (`@rari/core/guest`), and small
// build-time utilities.
//
// This entry pulls in Node built-ins; browser-safe modules are exposed as
// subpaths (`@rari/core/utils/type-guards`, `@rari/core/regex-constants`, …).
export * from './guest'
export * from './platform'
export * from './regex-constants'
export * from './router'
export * from './utils/dist-paths'
export * from './utils/path'
export * from './utils/regexp'
export * from './utils/type-guards'
export * from './utils/xml'
