# @rari/core

Framework-agnostic build contracts for [rari](https://rari.build).

rari is a Rust host (HTTP server, router, cache, V8 runtime pool) that renders through a _guest_ framework. The React adapter (`@rari/react`) is the built-in one; `@rari/qwik` is another. This package holds what every adapter shares and the host depends on:

- **Route manifest schema** — the JSON the Rust router reads from `dist/server/routes.json` (`AppRouteManifest` and friends in `@rari/core/router`).
- **Route scanner** — `generateAppRouteManifest(appDir, { conventions })` walks a file-system router directory. `RouteConventions` names the special files per framework (`page`/`layout`/… for React, `index`/`layout`/`404` for Qwik).
- **Metadata conventions** — app icon discovery, `robots`/`sitemap`/`feed` types, and the convention-file finder.
- **Binary resolution** — `@rari/core/platform` locates the rari binary (platform package, `PATH`, or Cargo).
- **Utilities** — path helpers, type guards, XML escaping and shared regexes, as individual subpaths so browser-safe modules stay separate from Node-only ones.

Adapters import from the subpaths (`@rari/core/router`, `@rari/core/utils/path`, …). The root entry re-exports everything and pulls in Node built-ins.
