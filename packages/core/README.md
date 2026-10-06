# @rari/core

Framework-agnostic build contracts for [rari](https://rari.build).

rari is a Rust host (HTTP server, router, cache, V8 runtime pool) that renders through a _guest_ framework. The React adapter (`@rari/react`) is the built-in one; `@rari/qwik` is another. This package holds what every adapter shares and the host depends on:

- **Route manifest schema** — the JSON the Rust router reads from `dist/server/routes.json` (`AppRouteManifest` and friends in `@rari/core/router`).
- **Route scanner** — `generateAppRouteManifest(appDir, { conventions, extensions })` walks a file-system router directory. `RouteConventions` names the special files per framework (`page`/`layout`/… for React, `index`/`layout`/`404` for Qwik); `extensions` says which files are modules (`.md`/`.mdx` pages, for example). On top of the names every framework gets the same route-file grammar (below).
- **Guest contract** — `@rari/core/guest` is what a framework's server bundle needs to run inside rari: the `GuestRequest` the host calls it with (URL, method, headers, body, client IP, and the host's `HostRoute` decision), the runtime's stream ops, and a `HostResponseSink` that speaks the host's response framing. Adapters build their platform shim on it so every guest streams, reports headers and honours routing the same way.
- **Metadata conventions** — app icon discovery, `robots`/`sitemap`/`feed` types, and the convention-file finder.
- **Binary resolution** — `@rari/core/platform` locates the rari binary (platform package, `PATH`, or Cargo).
- **Utilities** — path helpers, type guards, XML escaping and shared regexes, as individual subpaths so browser-safe modules stay separate from Node-only ones.

Adapters import from the subpaths (`@rari/core/router`, `@rari/core/utils/path`, …). The root entry re-exports everything and pulls in Node built-ins.

## Route-file grammar

A route directory names its files by role; the conventions say which base name plays which role. rari adds one grammar on top, identical for React, Qwik and any other adapter, so a page can choose the layouts that wrap it. With the React names (`page`, `layout`):

| File              | Meaning                                                     |
| ----------------- | ----------------------------------------------------------- |
| `page.tsx`        | the directory's page, wrapped by the default layout chain   |
| `page@wide.tsx`   | the page selects the `wide` layout variant                  |
| `page!.tsx`       | the page opts out of every layout                           |
| `layout.tsx`      | the directory's default layout                              |
| `layout-wide.tsx` | the `wide` layout variant; only pages that select it use it |
| `layout!.tsx`     | a top layout: the layouts above it are skipped              |

The same spellings apply to a framework's own names (`index@wide.tsx`, `404!.tsx`). The scanner records the selection on the manifest (`layout` / `skipLayouts` on a page, `name` / `skipParents` on a layout) and the Rust router resolves the chain per request: walking from the page's directory to the root, only the selected variant counts until it is found, after which each directory contributes its default layout; a top layout ends the walk. A variant that no page selects is not a layout, so a colocated `layout-helpers.tsx` stays a plain file.
