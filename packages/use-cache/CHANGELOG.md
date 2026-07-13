## [@rari/use-cache@0.15.0] - 2026-07-13

## Highlights

- **React Flight + Fizz RSC stack:** the runtime renders with official React Server Components: Flight for the RSC payload / client hydration path, and Fizz for HTML SSR and streaming. Wire-format naming and the client Flight bundle are aligned with that model.
- **Server actions on Flight:** actions use React Flight decode/encode end-to-end (`decodeAction` / `decodeReply`, form state, action refresh), with cookie-partitioned response cache keys after mutations.
- Experimental `'use cache'` with a native transform addon, memory/private storage, and remote backends (Redis and redb). Enable with `experimental.useCache` / `experimental.useCacheRemote`. APIs: `cacheLife`, `cacheTag`, `revalidateTag`, `revalidatePath`, `updateTag`.
- Production routing loads a unified `RoutesManifest` (`routes.json`) plus server component manifest; external framework client components such as `rari/image` resolve during SSR.
- MDX component registry via `defineMdxComponents` / `rari/mdx/registry` for shared MDX UI across the app.
- Image usage scanning runs in Rust (faster builds, same CLI surface).
- Dev HTML pretty-printing for local responses.
- Tooling: TypeScript 7 available side-by-side in the workspace catalog; create-rari-app templates target it. Node engine floor is `>=22.18.0`.

## Breaking Changes

- **Server action client entry:** `rari/runtime/actions` is removed. Import `callServer` from `rari/runtime/call-server` (the Vite plugin already injects this for transformed modules).
- **Form action endpoint:** `POST /_rari/form-action` is removed. Progressive enhancement and client actions use `POST /_rari/action` (and page POSTs) on the React Flight action path.
- **Node.js:** `engines.node` is now `>=22.18.0` (was `>=22.12.0`).
- **`'use cache'`:** remains behind `experimental.useCache` / `experimental.useCacheRemote`. Remote storage needs a matching `@rari/use-cache-*` native addon for your platform.

<!--
File naming (checked in order):
  1. --notes-file / RELEASE_NOTES_FILE
  2. .github/release-notes/<tag>.md
     `/` in scoped tags is replaced with `-` for the filename
     e.g. rari@0.15.0.md, v0.15.0.md, @rari-use-cache@0.15.0.md
  3. .github/release-notes/<version>.md
     e.g. 0.15.0.md (shared across release units)

Copy this template to one of those names before running `just release`.
Manual notes are prepended to git-cliff output for GitHub releases and
injected under the version heading in CHANGELOG.md.
-->

### 🚀 Features

- use cache remote (Redis) + native addon by @jarick
- add redb backend for use cache remote storage by @skiniks
- add cache storage modules for use-cache by @skiniks
- *(use-cache)* extend runtime parity, revalidation, and headers support by @skiniks
- migrate from @typescript/native-preview to TypeScript 7 🎉 by @skiniks

### 🐛 Bug Fixes

- *(use-cache)* gracefully handle unsupported platforms and improve error handling by @skiniks
- *(use-cache)* address review findings and consolidate RariGlobal access by @skiniks

### 🚜 Refactor

- *(cache-wrapper)* move cache-wrapper export to use-cache package by @skiniks
- *(use-cache)* extract deterministic-stringify into separate export by @skiniks
- *(use-cache)* remove internal runtime exports from public API by @skiniks
- simplify redis integration and update dependencies by @skiniks
- streamline rendering and module loading by introducing rsc-references by @skiniks
- rename rari dependencies and update CI workflows by @skiniks
- remove unused cache storage modules and update imports by @skiniks
- streamline cache storage retrieval and enhance mock backend functionality by @skiniks
- update import paths to use aliasing by @skiniks
- *(use-cache)* streamline error handling and logging in transform function by @skiniks
- *(use-cache)* remove quick-lru dependency and implement custom LRU cache by @skiniks

### ⚙️ Miscellaneous Tasks

- *(use-cache)* rename crate and package from use-cache-transform by @skiniks
- update GitHub Actions versions and integrate use-cache package builds by @skiniks
- update dependencies and configuration for use-cache distribution by @skiniks
- standardize product naming in descriptions and documentation by @skiniks
- update dependencies in Cargo.toml files across the project by @skiniks
- simplify build scripts by removing redundant clean command by @skiniks
- update dependencies in Cargo.toml files to use workspace references and ensure consistent versioning across Deno ecosystem by @skiniks
- update dependencies in Cargo.toml files to use workspace references for improved consistency and alignment with Deno ecosystem by @skiniks
- update justfile and Cargo.toml for consistency, add prepare_binaries tool for building platform-specific binaries and addons by @skiniks
- add CodSpeed benchmarks for use-cache transformer by @skiniks
- *(dependencies)* update various package versions by @skiniks
- *(dependencies)* update optional dependencies to use workspace references by @skiniks
- *(dependencies)* update optional dependencies to version 0.15.0 and adjust pnpm workspace configuration by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/use-cache-binaries@0.15.0...@rari/use-cache@0.15.0
