## [rari@0.15.0] - 2026-07-13

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

### ⚙️ Miscellaneous Tasks

- *(dependencies)* upgrade rari packages to version 0.15.0 in pnpm-lock.yaml, pnpm-workspace.yaml, and package.json by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.15.0...rari@0.15.0
## [0.15.0] - 2026-07-13

### 🚀 Features

- *(cache)* implement pluggable cache-handler system with memory and disk caching by @jarick
- *(use-cache-transform)* napi-rs addon for 'use cache' directive transformation by @jarick
- *(use-cache-transform)* napi-rs addon for 'use cache' directive transformation by @jarick
- add platform-specific native addon for use-cache-transform by @jarick
- *(module_loader)* implement component-aware version key generation by @skiniks
- use cache remote (Redis) + native addon by @jarick
- *(og-layout)* add space-evenly justify-content alignment option back by @skiniks
- add GitHub action for bundling React ESM vendor files by @skiniks
- *(cache)* implement static cache invalidation and enhance revalidation process by @skiniks
- *(rendering)* enhance RSC flight protocol validation and add tests by @skiniks
- *(config)* enhance environment configuration loading and cache control by @skiniks
- *(rendering)* improve error handling and add fizz chunk helper by @skiniks
- introduce MDX component registry and enhance MDX handling by @skiniks
- add redb backend for use cache remote storage by @skiniks
- add cache storage modules for use-cache by @skiniks
- introduce lazy loading for Deno extensions and enhance module structure by @skiniks
- enhance Deno extensions with new networking and TLS modules by @skiniks
- *(use-cache)* extend runtime parity, revalidation, and headers support by @skiniks
- *(image-scanner)* migrate image-scanner to rari crate from TS by @skiniks
- *(image-scanner)* implement alias regex caching and improve JSX alias processing by @skiniks
- *(server-build)* add support for external client components and improve manifest handling by @skiniks
- *(image-scanner)* add test for default import JSX in JS files and enhance image usage extraction by @skiniks
- *(rari)* enhance SSR client component resolution and streamline error boundary handling by @skiniks
- *(rari)* add type definitions for ErrorBoundaryWrapper in package.json by @skiniks
- *(rari)* implement server manifest loading and enhance routing with new RoutesManifest structure by @skiniks
- migrate from @typescript/native-preview to TypeScript 7 🎉 by @skiniks
- *(rari)* migrate server actions to React Flight protocol by @skiniks
- *(rari)* enhance action handling and form state management by @skiniks
- *(rari)* improve action flight response handling and form state management by @skiniks
- *(rari)* add response cache cookie partitioning and improve cache key generation by @skiniks
- *(rari)* enhance action form state management and request context handling by @skiniks
- *(rendering)* add support for preserving HTML comments containing '>' in pretty-printing by @skiniks

### 🐛 Bug Fixes

- *(core)* emit Cache-Control on static and synchronous render paths by @adambenhassen
- *(use-cache)* correct off-by-one in repoRoot path computation by @jarick
- *(use-cache-loader)* handle missing transformUseCacheModule gracefully by @skiniks
- *(api_routes)* validate HTTP status codes and improve response envelope handling by @skiniks
- *(server)* add vite-server WebSocket route without trailing slash by @skiniks
- *(og-rendering)* add missing text alignment options back by @skiniks
- improve module path resolution and rendering safety checks by @skiniks
- *(config)* trim remote cache URL and validate redis handler by @skiniks
- *(rendering)* enhance error handling in RSC streaming and hydration processes by @skiniks
- correct plugin configuration order in Rari config by @skiniks
- handle inline type imports in MDX component transformations by @skiniks
- clear file resolver cache on hot update to improve MDX handling by @skiniks
- *(use-cache)* address review findings and consolidate RariGlobal access by @skiniks
- *(image-scanner)* enhance JSON parsing with error handling for scanner output by @skiniks
- *(rari)* implement graceful shutdown handling and update dependencies by @skiniks
- *(rendering)* prevent panic in HTML tag parsing for multibyte characters by @skiniks
- *(rust-server)* improve process management for Rust server termination and ensure cleanup on exit by @skiniks
- *(request_context)* update error handling to use internal error for fetch singleflight join failures by @skiniks
- *(request_context)* ensure cleanup disarm is called correctly in request context caching logic by @skiniks

### 💼 Other

- *(use-cache)* add console.error logging to use-cache-transform plugin to diagnose CI cache miss by @jarick

### 🚜 Refactor

- *(runtime)* reorganize factory JS helpers into dedicated directory by @skiniks
- *(rsc)* consolidate RSC action tests into main module by @skiniks
- *(rsc)* reorganize action tests into dedicated module by @skiniks
- *(rsc)* allow disallowed clippy methods in action tests by @skiniks
- *(rsc)* allow additional clippy methods in action tests by @skiniks
- *(rsc)* consolidate RscElement and SuspenseBoundary into main module by @skiniks
- *(rsc)* allow disallowed clippy methods in core rendering tests by @skiniks
- *(rsc)* move parser and core rendering tests into inline modules by @skiniks
- migrate use-cache-transform to optional dependency with lazy loading by @skiniks
- *(cache)* consolidate cache exports and update import paths by @skiniks
- *(cache-wrapper)* move cache-wrapper export to use-cache package by @skiniks
- *(use-cache)* migrate from SHA1 to SHA256 hashing by @skiniks
- extract error and utility modules into separate crates by @skiniks
- *(rendering)* replace to_lowercase() with to_ascii_lowercase() for efficiency by @skiniks
- *(runtime)* remove module reload system and simplify runtime architecture by @skiniks
- *(runtime)* consolidate runtime module structure and simplify http adapter by @skiniks
- *(runtime)* remove http_adapter module and inline request/response handling by @skiniks
- *(server)* reorganize module structure and consolidate utilities by @skiniks
- *(vite)* consolidate proxy handlers and improve request forwarding by @skiniks
- *(transpile)* simplify version placeholder substitution check by @skiniks
- *(transpile)* improve version placeholder check with exact module matching by @skiniks
- *(runtime)* rename DenoRuntime to RariRuntime and reorganize factory module by @skiniks
- *(runtime)* reorganize factory and loader modules with utils submodules by @skiniks
- *(runtime)* rename deno_runtime parameter to js_runtime for clarity by @skiniks
- improve code formatting and consistency across codebase by @skiniks
- enforce workspace lints monorepo-wide, stricter linting rules and improve code quality by @skiniks
- improve code clarity and fix raw string handling by @skiniks
- improve code quality and clarify clippy lint reasoning by @skiniks
- improve path handling and error handling in core modules by @skiniks
- *(runtime)* extract and reuse source extension logic by @skiniks
- simplify redis integration and update dependencies by @skiniks
- *(runtime)* reorganize redis cache as extension module by @skiniks
- *(runtime)* extract redis cache extensions into separate function by @skiniks
- *(redis-cache)* remove async from get_redis_state and update error handling by @skiniks
- *(cache-config)* simplify URL trimming logic and improve readability by @skiniks
- *(config)* simplify URL validation check using method reference by @skiniks
- *(runtime)* migrate js helpers to typescript extension modules by @skiniks
- *(runtime)* revert to handle JsRuntime creation failures with graceful restart by @skiniks
- *(rari)* migrate rari module from JavaScript to TypeScript by @skiniks
- *(rari)* consolidate clippy allow attributes to module level by @skiniks
- *(runtime)* remove JavaScript entry point and consolidate imports by @skiniks
- reapply rustfmt formatting across codebase by @skiniks
- *(streaming)* simplify closure and add import row handling by @skiniks
- *(streaming)* extract boundary update and error handling into helper methods by @skiniks
- *(streaming)* remove unused boundary_rows_map and add error handling by @skiniks
- *(runtime)* migrate remaining extensions from JavaScript to TypeScript by @skiniks
- *(runtime)* improve type safety and correctness across extensions by @skiniks
- *(runtime)* improve type safety for cookie options and server function caching by @skiniks
- *(runtime)* avoid redundant JSON serialization in server function caching by @skiniks
- *(rsc)* flatten module structure and remove promise manager extension by @skiniks
- *(module_loader)* extract dependency extraction logic into utility function by @skiniks
- *(rsc)* extract React Server Components into dedicated workspace crate by @skiniks
- *(rsc)* flatten rendering module structure into main crate by @skiniks
- *(imports)* consolidate and optimize import statements across crates by @skiniks
- *(main)* move unix signal imports into platform-specific block by @skiniks
- migrate to official React Server Components with Fizz SSR by @skiniks
- consolidate absolute imports to relative paths by @skiniks
- *(router)* reorganize metadata route types and imports by @skiniks
- *(rendering)* remove Fizz streaming functionality and related code by @skiniks
- *(runtime)* streamline script execution and enhance timeout handling by @skiniks
- *(runtime)* replace hardcoded NODE_VERSION with deno_node constant by @skiniks
- streamline clippy allow attributes and update string formatting by @skiniks
- convert instance methods to static methods across runtime and rendering by @skiniks
- consolidate clippy attributes and enhance memory pressure constants by @skiniks
- convert mutable renderer references to immutable and optimize workspace configuration by @skiniks
- migrate clippy attributes from allow to expect by @skiniks
- migrate clippy attributes from allow to expect across codebase by @skiniks
- apply use_self clippy lint across codebase by @skiniks
- migrate clippy attributes from allow to expect in server module by @skiniks
- simplify extension trait implementations across runtime modules by @skiniks
- update type aliases for permissions container and improve file loading method by @skiniks
- rename wire format to flight protocol across rendering modules by @skiniks
- streamline rendering and runtime modules by introducing helper functions by @skiniks
- simplify FFI permission handling in AllowlistWebPermissions by @skiniks
- standardize error logging and improve code clarity by @skiniks
- unify logging practices and enhance utility function usage by @skiniks
- transition to asynchronous file operations across server modules by @skiniks
- enhance file existence checks with asynchronous operations by @skiniks
- convert path validation and file operations to asynchronous methods by @skiniks
- streamline rendering and module loading by introducing rsc-references by @skiniks
- remove streaming module and consolidate rendering logic by @skiniks
- enhance rendering and execution context handling by @skiniks
- improve HTML streaming and rendering logic by @skiniks
- enhance rendering logic and error handling by @skiniks
- improve client component error handling and type safety by @skiniks
- enhance HTML rendering and tag management by @skiniks
- streamline hydration utilities and component resolution by @skiniks
- enhance HTML stream management and error handling by @skiniks
- introduce namespace import transformation and client module proxy by @skiniks
- implement module analysis caching and enhance directive handling by @skiniks
- enhance module analysis and HTML import handling by @skiniks
- enhance module analysis and HTML import handling by @skiniks
- improve project context detection and caching mechanisms by @skiniks
- enhance binary spawning and module analysis by @skiniks
- improve module analysis by using original code for caching by @skiniks
- reorganize server structure and introduce new action handling by @skiniks
- reorganize CLI structure and enhance platform detection by @skiniks
- restructure router and vite module organization by @skiniks
- enhance MDX component handling and streamline imports by @skiniks
- introduce isMdxRegistryModuleId function for improved MDX module identification by @skiniks
- rename rari dependencies and update CI workflows by @skiniks
- move component and cache loaders by @skiniks
- *(cache)* move response cache tag merging inside conditional block by @skiniks
- *(cache)* improve async safety in response cache tag merging by @skiniks
- *(cache)* remove redundant pageCacheTags clearing in tag merging by @skiniks
- update import paths to use aliasing by @skiniks
- *(rari)* enhance shutdown signal handling for Unix and Windows platforms by @skiniks
- *(rari)* improve external client component resolution and enhance shutdown signal handling by @skiniks
- *(rari)* improve action script name generation for request context by @skiniks
- *(rari)* rename action script functions for clarity and improve naming conventions by @skiniks
- *(rari)* update file structure and improve import paths for better organization by @skiniks
- *(rendering)* enhance HTML output formatting and introduce pretty-printing for development by @skiniks
- *(rendering)* streamline global state management and improve component registration handling by @skiniks
- *(loader)* simplify global state initialization in loader and fallback module templates by @skiniks
- *(vite)* optimize global state initialization for server components in HMR by @skiniks
- *(rendering)* introduce renderer lock management for safe concurrent access and streamline rendering operations by @skiniks
- *(rendering)* simplify rendering logic by consolidating route rendering methods and improving concurrency handling by @skiniks

### ⚡ Performance

- optimize string allocations and comparisons across rendering pipeline by @skiniks

### 🎨 Styling

- *(api_routes)* suppress clippy disallowed_methods lint in test module by @skiniks
- fix indentation and remove unused imports across codebase by @skiniks

### 🧪 Testing

- *(rsc)* improve renderer test assertions and error handling by @skiniks
- *(path_url)* add comprehensive dot segment and relative path tests by @skiniks

### ⚙️ Miscellaneous Tasks

- consolidate deno_ast features and remove unused DefaultLoadingIndicator by @skiniks
- *(use-cache)* rename crate and package from use-cache-transform by @skiniks
- update dependencies and configuration for use-cache distribution by @skiniks
- *(templates,path-validation)* remove template placeholders and improve test reliability by @skiniks
- *(cargo)* consolidate package metadata to workspace level by @skiniks
- *(dependencies)* update various dependencies and improve project structure by @skiniks
- update dependencies in Cargo.toml files across the project by @skiniks
- refine tokio dependency configurations and update Node.js types in package.json by @skiniks
- refactor RSC-related modules by removing unused code and dependencies, including error handling and flight protocol validation by @skiniks
- simplify test expectations in core layout rendering by removing unused unwrap check by @skiniks
- add path mapping for use-cache module in TypeScript configuration by @skiniks
- streamline build and verification processes by removing conditional checks for use-cache addon by @skiniks
- refactor rendering modules by removing unused functions, updating sanitization logic, and enhancing flight protocol handling by @skiniks
- enhance rendering capabilities by adding Fizz and RSC scripts, improving streaming pipeline checks, and refactoring script loading logic by @skiniks
- enhance runtime operations by adding new node module checks, improving process shim functionality, and refining version placeholder handling by @skiniks
- streamline rendering and runtime processes by removing unused functions, enhancing stream completion handling, and refining error management in script execution by @skiniks
- enhance module loader functionality by adding component ID aliasing and refining component specifier registration by @skiniks
- refine component ID alias handling in module loader by removing redundant alias check and ensuring all aliases are removed from component specifiers by @skiniks
- simplify build scripts by removing redundant clean command by @skiniks
- remove rari_utils crate and move util back to main crate by @skiniks
- refactor rari crate by removing rari_rsc module and integrating its functionality directly into the main crate by @skiniks
- update dependencies in Cargo.toml files to use workspace references and ensure consistent versioning across Deno ecosystem by @skiniks
- update dependencies in Cargo.toml files to use workspace references for improved consistency and alignment with Deno ecosystem by @skiniks
- update justfile and Cargo.toml for consistency, add prepare_binaries tool for building platform-specific binaries and addons by @skiniks
- *(eslint, vite)* disable no-console rule for TypeScript files in bundle-react-esm directory by @skiniks
- *(dependencies)* update various package versions by @skiniks
- *(dependencies)* update various package versions by @skiniks
- *(dependencies)* update @rari/use-cache to version 0.15.0 in package.json and pnpm-lock.yaml by @skiniks


### 🆕 New Contributors

- @adambenhassen made their first contribution

**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.14.12...v0.15.0
## [0.14.12] - 2026-06-16

### 🚀 Features

- *(cli)* add clean command to rari CLI (#218) by @skiniks

### 🚜 Refactor

- *(rsc)* reorganize module structure and rename wire_format to flight by @skiniks
- *(error)* change error metadata type from Option<ErrorMetadata> to Option<Box<ErrorMetadata>> by @skiniks
- *(actions)* move server actions to rsc module by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.14.11...v0.14.12
## [0.14.11] - 2026-06-12

### 🚀 Features

- *(ssr)* implement server-side rendering for client components by @skiniks
- *(ssr)* add support for module imports in RSC rendering by @skiniks
- *(ssr)* enhance server-side rendering with improved error handling and component stubs by @skiniks
- *(module-loader)* add stubs for headers, image, and client components by @skiniks

### 🐛 Bug Fixes

- *(ssr)* render client component children to HTML instead of empty placeholders by @skiniks
- *(ssr)* enhance component loading and rendering logic by @skiniks
- *(ssr)* refine component type checks in rendering logic by @skiniks
- *(ssr)* enhance rendering logic for suspense and client components by @skiniks
- *(ssr)* improve RSC value handling in rendering logic by @skiniks
- *(ssr)* streamline RSC value checks in rendering logic by @skiniks
- *(rendering)* refine RSC string validation logic to support new prefixes by @skiniks
- *(rendering)* update cloneElement logic to handle children props correctly by @skiniks

### 🚜 Refactor

- *(ssr)* centralize RSC children type checks in rendering logic by @skiniks
- *(proxy)* reorganize proxy utilities and introduce new header processing functions by @skiniks
- *(rendering)* simplify suspense handling in RSC logic by @skiniks
- *(module-loader)* streamline stub selection logic for module loading by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.14.9...v0.14.11
## [rari@0.14.9] - 2026-06-11

### ⚙️ Miscellaneous Tasks

- add version 0.0.0 to deploy and logger packages, downgrade rari to version 0.14.8 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.14.9...rari@0.14.9
## [0.14.9] - 2026-06-11

### 🚀 Features

- *(metadata)* add alternates metadata support for feed generation by @skiniks

### 🚜 Refactor

- *(metadata)* enhance canonical link handling and improve metadata merging logic by @skiniks
- *(router)* reorganize imports and enhance utility functions by @skiniks
- *(server)* streamline server component eligibility checks and enhance file resolution logic by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.14.8...v0.14.9
## [rari@0.14.8] - 2026-06-08

### 🐛 Bug Fixes

- *(vite-plugin)* update warning suppression for dynamic imports in dist files by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.14.8...rari@0.14.8
## [0.14.8] - 2026-06-08

### 🚀 Features

- add Next.js App Router template.tsx (per-segment re-mount) to Rari by @jarick

### 🐛 Bug Fixes

- *(rsc)* await renderServerComponent in traverseReactElement for proper async handling by @skiniks
- *(rsc-client-runtime)* disable eslint rule for process global usage by @skiniks

### 🚜 Refactor

- *(routing)* remove deprecated is_layout_ancestor method by @skiniks
- *(vite)* optimize directive utils with char code comparisons by @skiniks
- *(vite)* consolidate directive detection and standardize char code constants by @skiniks
- *(vite)* optimize server component detection and cache html entry imports by @skiniks
- *(rsc)* extract React symbol constants for improved maintainability by @skiniks
- *(rsc)* rename REACT_FRAGMENT_SYMBOL to REACT_FRAGMENT_TYPE for consistency by @skiniks
- *(rendering)* streamline error handling and remove unused components by @skiniks
- *(rendering)* enhance fallback handling and streamline client component registration by @skiniks
- *(rsc)* simplify global suspense initialization and remove unused status response interface by @skiniks
- *(rsc)* enhance async component detection and error handling in traverseReactElement by @skiniks
- *(rendering)* update component rendering to support async operations by @skiniks
- *(error-handling)* streamline error display and improve error boundary logic by @skiniks

### ⚙️ Miscellaneous Tasks

- bump version to 0.14.8 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.14.7...v0.14.8
## [rari@0.14.7] - 2026-06-07

### 🚀 Features

- *(router)* implement route groups and additional paths for layouts by @jarick
- *(router)* add duplicate route detection and support API routes in groups by @skiniks
- *(server-build)* add component export detection for server components by @skiniks
- *(regex)* improve export detection patterns for functions and classes by @skiniks
- *(regex)* refine export detection patterns for better accuracy by @skiniks
- *(regex)* expand const function export pattern to match function declarations by @skiniks
- *(regex)* improve exported const function pattern to match arrow functions with parameters by @skiniks
- *(regex)* refine const export pattern to match arrow functions correctly by @skiniks

### 🐛 Bug Fixes

- *(proxy)* add debug logging when executor not found by @skiniks

### 🚜 Refactor

- improve runtime path resolution across Rust and TypeScript modules by @skiniks
- improve runtime path resolution and internal file detection by @skiniks
- *(cache)* replace custom LRU implementation with lru crate by @skiniks

### 🧪 Testing

- *(http_utils)* add comprehensive content type mapping tests by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.14.6...rari@0.14.7
## [rari@0.14.6] - 2026-06-04

### ⚡ Performance

- *(server,router)* parallelize cache warmup and optimize route resolution by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.14.4...rari@0.14.6
## [rari@0.14.4] - 2026-06-03

### 🚀 Features

- add CSS module support for server-rendered pages by @jarick

### 🚜 Refactor

- *(router)* make componentId optional and reorder CSS modules config by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.14.3...rari@0.14.4
## [rari@0.14.3] - 2026-06-02

### 🐛 Bug Fixes

- reset suspense.currentBoundaryId between renders to prevent async components from returning null during RSC navigation by @jarick

### 🚜 Refactor

- extract suspense type checking into shared utility by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.14.2...rari@0.14.3
## [rari@0.14.2] - 2026-06-01

### 🐛 Bug Fixes

- Windows path compatibility for tests and Rust path resolver by @jarick
- streaming nested suspense lazy resolution by @jarick

### 🎨 Styling

- *(path)* reformat file_path_to_component_id method by @skiniks


### 🆕 New Contributors

- @jarick made their first contribution

**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.14.1...rari@0.14.2
## [rari@0.14.1] - 2026-05-26

### 🚀 Features

- *(rsc)* add cache warmup for static routes and update route matching by @skiniks

### 🐛 Bug Fixes

- *(rsc)* await element resolution and wrap state updates in transition by @skiniks
- *(rsc)* handle streaming responses without transition wrapping by @skiniks

### 🚜 Refactor

- *(rsc)* remove unused export regex patterns and simplify manifest by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.14.0...rari@0.14.1
## [rari@0.14.0] - 2026-05-26

### 🚀 Features

- *(server)* remove rate limiting and spam blocking middleware by @skiniks

### 🐛 Bug Fixes

- *(rsc)* handle double-dollar escape sequences and remove redundant escaping by @skiniks
- *(rsc)* escape props values before JSON serialization in wire format by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.13.12...rari@0.14.0
## [rari@0.13.12] - 2026-05-26

### 🚀 Features

- *(sitemap)* add alias resolution and Vite dev server CORS support by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.13.11...rari@0.13.12
## [rari@0.13.11] - 2026-05-24

### 🚀 Features

- *(rsc)* optimize non-streaming route rendering with inline RSC payload by @skiniks

### 🐛 Bug Fixes

- *(security)* update COEP header from require-corp to credentialless by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.13.10...rari@0.13.11
## [rari@0.13.9] - 2026-05-24

### ⚙️ Miscellaneous Tasks

- refactor snapshot generation and consolidate dependencies by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.13.8...rari@0.13.9
## [rari@0.13.8] - 2026-05-24

### 🚀 Features

- *(runtime)* embed deno_node lazy-loaded sources at compile time by @skiniks
- *(runtime)* embed deno_node lazy-loaded sources at compile time by @skiniks
- *(runtime)* embed V8 snapshot at compile time by @skiniks

### 🐛 Bug Fixes

- add binary runtime initialization check and deployment fixes by @skiniks

### ⚙️ Miscellaneous Tasks

- remove binary runtime initialization check by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.13.7...rari@0.13.8
## [rari@0.13.7] - 2026-05-23

### 🐛 Bug Fixes

- Remove non-streaming rendering path from layout core renderer by @skiniks
- *(runtime)* resolve V8 isolate slot conflict in deno_node synthetic modules by @skiniks
- *(runtime)* add withFileTypes option support to readdir stub by @skiniks
- *(runtime)* initialize globalThis['~rsc'] object before module assignment by @skiniks

### 🚜 Refactor

- *(runtime)* consolidate module loader stubs and reorganize node initialization by @skiniks
- *(runtime)* simplify module stub generation by @skiniks
- *(runtime)* remove init_napi extension and consolidate napi initialization by @skiniks
- *(runtime)* remove client reference bridge registration by @skiniks

### ⚙️ Miscellaneous Tasks

- refactor release workflow permissions and enhance buffer encoding support by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.13.6...rari@0.13.7
## [rari@0.13.4] - 2026-05-07

### ⚙️ Miscellaneous Tasks

- *(rari)* downgrade version to 0.13.2 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.13.3...rari@0.13.4
## [rari@0.13.2] - 2026-04-29

### 🐛 Bug Fixes

- *(rsc)* improve component registration and HMR handling by @skiniks

### ⚙️ Miscellaneous Tasks

- update dependencies to latest versions by @skiniks
- bump rari version to 0.13.2 by @skiniks
- *(rari)* bump rari-binaries to 0.13.2 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.13.1...rari@0.13.2
## [rari@0.13.1] - 2026-04-24

### ⚙️ Miscellaneous Tasks

- update dependencies and standardize health check endpoint by @skiniks
- update rari version to 0.13.1 in Cargo.toml by @skiniks
- update optionalDependencies to version 0.13.1 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.13.0...rari@0.13.1
## [rari@0.13.0] - 2026-04-21

### 🚀 Features

- *(build, ci)* add rari-win32-arm64 by @Master-Hash

### ⚙️ Miscellaneous Tasks

- update package version to 0.13.0 in Cargo.toml by @skiniks
- update dependencies in pnpm-workspace.yaml by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.12.2...rari@0.13.0
## [rari@0.12.2] - 2026-04-18

### 🐛 Bug Fixes

- *(rsc)* mark Image component as client reference on server by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.12.1...rari@0.12.2
## [rari@0.12.1] - 2026-04-16

### 🐛 Bug Fixes

- *(rsc)* improve client component resolution and simplify external imports by @skiniks
- *(rsc)* simplify default export resolution in React Flight client config by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump version to 0.12.1 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.12.0...rari@0.12.1
## [rari@0.12.0] - 2026-04-16

### 🚀 Features

- *(rsc)* optimize boundary update streaming with progressive HTML rendering by @skiniks
- *(rsc)* refactor streaming and RSC rendering with React Flight client integration by @skiniks
- *(rsc)* implement hexadecimal row ID encoding for wire format by @skiniks
- *(rsc)* implement hexadecimal row reference parsing and streaming bridge by @skiniks
- *(rsc)* refactor wire format encoding and streaming bridge initialization by @skiniks
- *(rsc)* implement streaming bridge and hexadecimal chunk encoding by @skiniks
- *(rsc)* improve wire format validation and streaming cleanup by @skiniks
- *(rsc)* add export name tracking to component registry by @skiniks
- *(rsc)* enhance client component registration and reference parsing by @skiniks
- *(rsc)* improve client component resolution and error handling by @skiniks
- *(rsc)* add export name tracking to client component loader by @skiniks
- *(rsc)* improve client reference detection and navigation abort handling by @skiniks
- *(router)* simplify navigation handler and fix response URL handling by @skiniks

### 🐛 Bug Fixes

- *(rsc)* normalize path separators in component ID generation by @skiniks
- *(rsc)* normalize path separators in wire format serializer by @skiniks
- *(rsc)* normalize path separators in promise resolver by @skiniks
- *(rsc)* normalize path separators in component registry operations by @skiniks
- *(rsc)* normalize path separators in client component resolution by @skiniks
- *(rsc)* add fallback resolution for non-normalized component IDs by @skiniks
- *(rsc)* normalize path separators in server build component ID resolution by @skiniks
- *(rsc)* improve render generation error handling and simplify test timeouts by @skiniks
- *(rsc)* normalize path separators in component resolution and improve error handling by @skiniks
- *(rsc)* normalize path separators in component registry operations by @skiniks
- *(rsc)* improve component registry cleanup and streaming initialization by @skiniks
- *(rsc)* normalize path separators in dependency graph lookups and fix hex digit validation by @skiniks
- *(rsc)* extract path normalization into reusable method by @skiniks
- *(rsc)* improve component registry cleanup and streaming initialization by @skiniks
- *(rsc)* normalize path separators in dependency graph lookups and fix hex digit validation by @skiniks
- *(rsc)* improve component registry cleanup and hex digit validation by @skiniks

### 🚜 Refactor

- *(rsc)* extract render generation retrieval and import row parsing by @skiniks
- *(rsc)* extract path normalization into reusable normalize_id method by @skiniks

### 🧪 Testing

- *(rsc)* add comprehensive path normalization tests for component registry by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump version to 0.12.0 by @skiniks
- *(rari)* bump optional dependencies to 0.12.0 by @skiniks

### ◀️ Revert

- back to working state at 55be557f by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.11.0...rari@0.12.0
## [rari@0.11.0] - 2026-04-03

### 🚀 Features

- *(rsc)* implement streaming boundary updates with DOM rendering by @skiniks
- *(cli)* add vite-plus detection and support dual vite/vite-plus usage by @skiniks
- *(rsc)* implement batch promise resolution with concurrent script execution by @skiniks
- *(rsc)* enhance HTML rendering with improved attribute handling and streaming by @skiniks
- *(rsc)* improve error handling and async component rendering by @skiniks
- *(rsc)* improve RSC rendering with better attribute validation and cookie handling by @skiniks
- *(rsc)* optimize streaming boundary rendering and promise handling by @skiniks
- *(rsc)* improve suspense boundary handling and streaming updates by @skiniks
- *(rsc)* improve streaming and promise handling with better serialization by @skiniks
- *(rsc)* improve concurrent script execution and promise handling by @skiniks
- *(rsc)* improve element serialization and attribute handling by @skiniks
- *(rsc)* improve security, accessibility, and HTML rendering by @skiniks
- *(rsc)* improve streaming event handling and boundary content rendering by @skiniks
- *(rsc)* upgrade deno dependencies and improve boundary context handling by @skiniks
- *(rsc)* enhance component context and improve null checks by @skiniks
- *(rsc)* improve boundary handling and payload validation by @skiniks
- *(rsc)* improve suspense boundary handling and navigation state management by @skiniks
- *(rsc)* improve async boundary handling and add circular reference detection by @skiniks
- *(rsc)* improve suspense boundary handling and RSC wire format parsing by @skiniks
- *(rsc)* improve suspense boundary children processing and return structure by @skiniks
- *(rsc)* add stream completion marker to RSC streaming response by @skiniks
- *(rsc)* add cookie validation and RSC fallback rendering by @skiniks
- *(rsc)* add stream completion check to suspense boundary rendering by @skiniks
- *(rsc)* refactor lazy promise resolution with circular reference detection by @skiniks
- *(runtime)* refactor request context management and improve origin validation by @skiniks
- *(rsc)* refactor rendering pipeline with circular reference detection by @skiniks
- *(rsc)* extract lazy promise resolution into dedicated method by @skiniks
- *(rsc)* refactor streaming setup and improve origin validation by @skiniks
- *(rsc)* refactor lazy promise resolution and request context handling by @skiniks
- *(rsc)* refactor stream context handling and improve directive parsing by @skiniks
- *(rsc)* improve cache validation and origin pattern matching by @skiniks
- *(rsc)* refactor request context lifecycle and improve origin validation by @skiniks
- *(rsc)* refactor directive parsing and stream context handling by @skiniks
- *(rsc)* refactor rendering pipeline and improve error handling by @skiniks
- *(rsc)* extract streaming enablement constant and simplify wire format validation by @skiniks
- *(rsc)* strengthen error handling and extract regex keywords constant by @skiniks
- *(rsc)* add streaming disable capability and improve render error handling by @skiniks
- *(rsc)* refactor streaming pipeline and improve error handling by @skiniks
- *(rsc)* refactor rendering pipeline with cleanup handlers by @skiniks
- *(rsc)* implement streaming reference counting for lifecycle management by @skiniks

### 🐛 Bug Fixes

- *(rsc)* update element array length validation for RSC protocol by @skiniks

### 🚜 Refactor

- remove CSRF protection and add search functionality by @skiniks

### 🎨 Styling

- *(rsc)* fix indentation in layout rendering error handling by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.25...rari@0.11.0
## [rari@0.10.25] - 2026-03-26

### 🚀 Features

- *(rsc)* add attribute name validation and refactor HTML escaping by @skiniks
- *(rsc)* enhance HTML rendering security with tag and attribute validation by @skiniks
- *(rsc)* add automatic px unit suffix for numeric style values by @skiniks
- *(rsc)* distinguish boolean HTML attributes from aria/content attributes by @skiniks

### 🐛 Bug Fixes

- *(rsc)* skip non-numeric style values without unit suffix by @skiniks

### 🚜 Refactor

- *(rsc)* revert wire_format serializer to address memory regressions by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.24...rari@0.10.25
## [rari@0.10.24] - 2026-03-25

### 🚀 Features

- *(rsc)* add automatic px unit suffix for numeric CSS values by @skiniks

### 🚜 Refactor

- *(rsc)* consolidate HTML escaping functions and add tag validation by @skiniks
- *(rsc)* improve event handler detection with character-based validation by @skiniks

### ⚡ Performance

- *(rsc)* optimize JSON serialization and HTML rendering by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump version to 0.10.24 by @skiniks
- *(rari)* bump optional dependencies to 0.10.24 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.23...rari@0.10.24
## [rari@0.10.23] - 2026-03-24

### 🐛 Bug Fixes

- resolve Windows path handling issues in module resolution by @skiniks

### 🚜 Refactor

- *(vite)* extract use client directive validation logic by @skiniks
- *(vite)* reorder comment handling logic in HMR coordinator by @skiniks
- *(vite)* improve comment handling and test organization by @skiniks
- *(vite)* reorder comment skip logic in HMR coordinator by @skiniks
- *(vite)* simplify inline block comment stripping logic by @skiniks
- *(vite)* improve comment stripping and enhance test assertions by @skiniks
- *(vite)* handle incomplete block comments in directive detection by @skiniks

### 🧪 Testing

- *(vite)* add comprehensive unit tests for vite plugin by @skiniks
- *(vite)* add input validation and improve test coverage by @skiniks
- *(vite)* enhance validation and cross-platform path handling by @skiniks
- *(vite)* improve block comment handling and cross-platform paths by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump version to 0.10.23 by @skiniks
- *(rari)* bump optional dependencies to 0.10.23 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.22...rari@0.10.23
## [rari@0.10.22] - 2026-03-19

### 🐛 Bug Fixes

- *(module-loader)* refactor React stub resolution logic by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.21...rari@0.10.22
## [rari@0.10.21] - 2026-03-19

### 🐛 Bug Fixes

- resolve removed React stub regression by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.20...rari@0.10.21
## [rari@0.10.20] - 2026-03-18

### 🚀 Features

- *(fetch)* add cache tags support to fetch operations by @skiniks

### 🐛 Bug Fixes

- add defensive checks and improve path resolution logic by @skiniks

### 🚜 Refactor

- reorganize deployment and streaming setup code by @skiniks
- consolidate deployment and utility functions by @skiniks
- *(runtime)* consolidate React globals and streaming setup by @skiniks
- *(module_reload)* remove stats and history tracking by @skiniks
- consolidate React globals and improve fetch cache handling by @skiniks
- improve React setup and error handling across runtime by @skiniks
- improve React JSX handling and image optimization logic by @skiniks
- improve JSX handling, fetch caching, and proxy utilities by @skiniks
- *(ClientRouter)* move route ref update before state change by @skiniks
- optimize React JSX handling and image component logic by @skiniks
- add defensive type checks and improve null handling by @skiniks
- consolidate JSX delegate creation and improve component resolution by @skiniks
- improve JSX delegate creation and component ID resolution by @skiniks
- add defensive checks and improve component resolution by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.19...rari@0.10.20
## [rari@0.10.19] - 2026-03-15

### 🚀 Features

- *(rari)* refactor RSC payload handling and suspense management by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump version to 0.10.19 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.18...rari@0.10.19
## [rari@0.10.18] - 2026-03-14

### 🚀 Features

- *(rari)* optimize RSC payload ref handling and add CLI integration tests by @skiniks
- *(rari)* improve RSC rendering and add CLI utility exports by @skiniks
- *(rari)* improve stale content handling and navigation state tracking by @skiniks
- *(rari)* refactor RSC payload parsing and freshness tracking by @skiniks
- *(rari)* add type safety and improve RSC payload ref handling by @skiniks
- *(rari)* improve component validation and RSC error tracking by @skiniks
- *(rari)* extract and track freshness token from RSC payload by @skiniks

### ⚙️ Miscellaneous Tasks

- *(typescript)* centralize TypeScript configuration by @skiniks
- *(rari)* bump version to 0.10.18 by @skiniks
- *(rari)* bump binary dependencies to 0.10.18 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.17...rari@0.10.18
## [rari@0.10.17] - 2026-03-14

### 🚀 Features

- *(rari)* add error boundary support and expand e2e test coverage by @skiniks
- *(rari)* improve error boundary handling and static file serving by @skiniks
- *(rari)* add client directive to error boundary and improve component ID generation by @skiniks
- *(rari)* add lifecycle management to error boundary component by @skiniks
- *(rari)* remove mount checks and improve use client directive handling by @skiniks
- *(rsc)* improve component rendering and fix edge cases by @skiniks
- *(rsc)* improve component rendering stability and HMR cache handling by @skiniks
- *(rsc)* optimize component rendering with memoized callbacks and lazy content loading by @skiniks
- *(rsc)* improve error handling, promise cleanup, and component rendering by @skiniks
- *(rsc)* improve streaming error format and optimize cache invalidation by @skiniks
- *(rari)* migrate to vite-plus CLI and improve RSC root call handling by @skiniks
- *(rari)* improve RSC payload handling and suspense cleanup by @skiniks
- *(rari)* improve component rendering and async loading by @skiniks
- *(rari)* improve type safety and refactor async component loading by @skiniks

### 🚜 Refactor

- *(rari)* optimize routing and middleware with extracted utilities by @skiniks

### ⚙️ Miscellaneous Tasks

- migrate from tsdown to vite-plus and consolidate build tooling by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.16...rari@0.10.17
## [rari@0.10.16] - 2026-03-12

### 🚀 Features

- *(runtime)* remove suppressions + update to Vite 8 by @skiniks
- *(runtime)* improve export collision detection and skip library component registration by @skiniks
- *(runtime)* add skipGlobalBinding parameter to component registration by @skiniks
- *(runtime)* improve component collision detection and client marking by @skiniks
- *(runtime)* improve client component marking for library components by @skiniks

### 🐛 Bug Fixes

- *(runtime)* handle component registration collisions and library utilities by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.15...rari@0.10.16
## [rari@0.10.15] - 2026-03-11

### 🐛 Bug Fixes

- *(runtime)* improve error handling and safety checks in JS helpers by @skiniks
- *(runtime)* improve component loading and JSON serialization by @skiniks
- *(runtime)* add serialization error handling and safety checks by @skiniks
- *(runtime)* improve component loading safety and JSON serialization by @skiniks
- *(runtime)* improve server action registration safety and namespacing by @skiniks
- *(runtime)* improve server function namespacing and lookup by @skiniks
- *(runtime)* enhance server function resolution and cleanup safety by @skiniks
- *(runtime)* improve module cleanup and component loading safety by @skiniks
- *(runtime)* enhance server function resolution and component cleanup by @skiniks
- *(runtime)* enhance server function registry and error handling by @skiniks
- *(runtime)* improve server function cleanup and error handling by @skiniks
- *(runtime)* improve component path detection and serialization error logging by @skiniks

### 🚜 Refactor

- *(rari)* reorganize main entry point exports by @skiniks
- *(runtime)* restructure JS runtime helpers into modular handlers by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump version to 0.10.15 by @skiniks
- *(rari)* bump optional dependencies to 0.10.15 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.14...rari@0.10.15
## [rari@0.10.14] - 2026-03-10

### 🚜 Refactor

- *(rsc)* replace double underscore globals with bracket notation by @skiniks
- *(rsc)* replace double underscore globals with tilde by @skiniks
- *(rsc)* reorganize global namespace under ~rari object by @skiniks
- *(rsc)* deduplicate rsc namespace initialization and improve suspense handling by @skiniks
- *(rsc)* simplify client components namespace access by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.13...rari@0.10.14
## [rari@0.10.13] - 2026-03-07

### 🐛 Bug Fixes

- *(runtime)* update keyseq timeout to use Option type by @skiniks
- *(runtime)* downgrade libc and simplify interrupt handling by @skiniks
- *(runtime)* remove WindowResized from readline error handling by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.12...rari@0.10.13
## [rari@0.10.12] - 2026-03-06

### 🐛 Bug Fixes

- *(cli)* improve main module detection for esm entry points by @skiniks
- *(cli)* improve main module detection by @skiniks
- *(cli)* resolve symlinks in module detection by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.11...rari@0.10.12
## [rari@0.10.11] - 2026-03-04

### 🐛 Bug Fixes

- *(cli)* simplify bun package executor to use bunx directly by @skiniks
- *(cli)* improve cross-platform spawn handling by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.10...rari@0.10.11
## [rari@0.10.10] - 2026-03-01

### 🚀 Features

- *(fetch-cache)* add timeout option to global fetch configuration by @skiniks

### 🐛 Bug Fixes

- *(cli)* improve bun package executor handling for cross-platform compatibility by @skiniks
- *(cli)* refactor bun package executor to improve cross-platform compatibility by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.9...rari@0.10.10
## [rari@0.10.9] - 2026-02-26

### 🚀 Features

- *(fetch-cache)* integrate fetch caching into runtime initialization by @skiniks
- *(runtime)* add request context cleanup and optimize fetch caching by @skiniks
- *(runtime)* refactor request context cleanup and optimize fetch caching by @skiniks
- *(fetch,routing)* improve cache key generation and request handling by @skiniks
- *(routing,streaming)* simplify component ID derivation and improve streaming test coverage by @skiniks
- *(fetch,testing)* improve cache key generation and add runtime helpers by @skiniks

### 🐛 Bug Fixes

- *(fetch,streaming)* improve cache handling and clarify test descriptions by @skiniks
- *(routing)* use cow_replace for efficient path normalization by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.8...rari@0.10.9
## [rari@0.10.8] - 2026-02-25

### 🚀 Features

- *(router)* add public router API with navigate function and hooks by @skiniks
- *(router)* add route segment matching and improve parameter extraction by @skiniks
- *(router)* export navigate getter and improve router provider by @skiniks
- *(router)* add navigate deregistration and cleanup on unmount by @skiniks

### 🐛 Bug Fixes

- *(router)* preserve hash in history state and handle hash-based scrolling by @skiniks
- *(router)* improve navigation registration and hash scrolling behavior by @skiniks
- *(router)* refactor parameter extraction and improve navigate registration by @skiniks
- *(router)* add spacing between navigate registration functions by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump optional dependencies to 0.10.8 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.7...rari@0.10.8
## [rari@0.10.7] - 2026-02-24

### 🐛 Bug Fixes

- *(rsc)* add fragment element support for RSC rendering by @skiniks
- *(rsc)* improve null handling and empty array detection in streaming by @skiniks
- *(rsc)* use nullish coalescing for children fallback logic by @skiniks
- *(rsc)* improve children null handling in streaming initialization by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.6...rari@0.10.7
## [rari@0.10.6] - 2026-02-24

### 🐛 Bug Fixes

- *(runtime)* improve DOM manipulation safety and performance by @skiniks
- *(runtime)* enhance DOM safety and add HTML sanitization by @skiniks
- *(runtime)* normalize SVG element names to lowercase by @skiniks
- *(runtime)* add error handling for client component hydration by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.5...rari@0.10.6
## [rari@0.10.5] - 2026-02-24

### 🐛 Bug Fixes

- *(AppRouterProvider)* remove redundant cache control headers by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.4...rari@0.10.5
## [rari@0.10.4] - 2026-02-23

### 🚀 Features

- *(http-headers)* add Vary header to all response types by @skiniks
- *(http-headers)* add Vary header to cached responses and fix fragment symbol scope by @skiniks
- *(http-headers)* merge Vary headers from cached responses by @skiniks

### 🐛 Bug Fixes

- *(http-headers)* handle wildcard Vary header early in merge logic by @skiniks

### 🚜 Refactor

- *(server)* simplify nested conditionals by @skiniks
- *(http-headers)* extract Vary header merging logic into utility function by @skiniks
- *(http-headers)* use cached_key for Vary header sorting and merge in 304 responses by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.3...rari@0.10.4
## [rari@0.10.3] - 2026-02-23

### 🐛 Bug Fixes

- *(rendering,router)* correct script type and stabilize navigation ref by @skiniks
- *(router)* use useLayoutEffect for navigate ref synchronization by @skiniks
- *(router)* move ref assignments into useLayoutEffect with cleanup by @skiniks
- *(router)* add null safety to navigateRef and cleanup on unmount by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.2...rari@0.10.3
## [rari@0.10.2] - 2026-02-22

### 🐛 Bug Fixes

- *(router)* ensure stale window and navigate refs update correctly with dependency changes by @skiniks
- *(router)* refactor effect dependencies and improve test reliability by @skiniks
- *(router)* separate event listener registration from stale window effect by @skiniks
- *(rendering)* reorder metadata and asset injection for correct HTML structure by @skiniks
- *(rsc,router,sentry)* sanitize JSON parsing and secure boundary updates by @skiniks
- *(router,sentry)* improve JSON parsing resilience and error handling by @skiniks
- *(router)* improve JSON parsing resilience in route info client by @skiniks
- *(rendering,router,runtime,sentry)* improve security and error handling by @skiniks
- *(rendering,router,runtime,sentry)* enhance security and error handling by @skiniks
- *(rendering,rsc-client,sentry)* enhance security and accessibility by @skiniks
- *(rendering,rsc-client,sentry)* enhance security and sanitization by @skiniks

### 🚜 Refactor

- *(rendering)* consolidate HTML escaping and improve metadata injection by @skiniks
- *(rendering)* consolidate HTML escaping and improve attribute safety by @skiniks
- *(rsc)* replace innerHTML with DOM API in HMR error overlay by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump version to 0.10.2 by @skiniks
- *(rari)* bump optional dependencies to 0.10.2 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.1...rari@0.10.2
## [rari@0.10.1] - 2026-02-20

### 🚀 Features

- *(caching)* consolidate cache control configuration into server config by @skiniks
- *(rsc)* add promise caching and improve lazy component resolution for HMR by @skiniks
- *(rsc)* implement promise caching and improve lazy component resolution by @skiniks
- *(hmr)* improve error handling and promise cleanup in HMR and RSC runtime by @skiniks
- *(hmr)* improve server URL resolution and add type safety by @skiniks
- *(hmr)* improve error handling and response parsing in HMR runtime by @skiniks
- *(rsc-client-runtime)* improve server URL resolution and add response validation by @skiniks
- *(metadata-injection)* add default charset and viewport meta tags by @skiniks
- *(rsc-layout)* add cache clearing on layout render by @skiniks
- *(rsc-layout)* improve promise caching with unique identifiers by @skiniks

### 🐛 Bug Fixes

- *(server)* validate cache-control header values during config parsing by @skiniks
- *(server)* improve cache-control validation error messages and add type checking by @skiniks
- *(server)* improve path pattern matching and add configurable base path support by @skiniks
- *(rsc)* improve response validation logic in HMR reload handler by @skiniks

### 🎨 Styling

- *(rsc-client-runtime)* improve code formatting and extract server URL resolution by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.10.0...rari@0.10.1
## [rari@0.10.0] - 2026-02-18

### 🚀 Features

- *(og)* add SVG rendering support for open graph images by @skiniks

### 🐛 Bug Fixes

- *(og)* improve SVG rendering precision and component handling by @skiniks
- *(og)* handle async components and rendering errors gracefully by @skiniks
- *(og)* return null for non-function component resolution by @skiniks

### 🚜 Refactor

- *(og)* extract component resolution logic into dedicated method by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.9.3...rari@0.10.0
## [rari@0.9.3] - 2026-02-17

### 🚀 Features

- *(rsc-rendering)* improve RSC shell rendering and payload handling by @skiniks

### 🐛 Bug Fixes

- *(rate_limit)* correct default enabled state in test assertion by @skiniks

### 🚜 Refactor

- extract regex patterns to constants for reusability by @skiniks
- *(node)* remove redundant PATH_SLASHES_REGEX pattern by @skiniks
- optimize regex patterns and simplify conditional logic by @skiniks
- improve regex patterns and fix capture group references by @skiniks
- improve path handling and rename regex constants for clarity by @skiniks
- optimize path resolution and regex pattern ordering by @skiniks
- improve regex patterns and parameter handling by @skiniks
- improve parameter extraction and remove unused path alias by @skiniks
- *(rsc-rendering)* simplify RSC payload handling and element rendering by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump version to 0.9.3 by @skiniks
- *(rari)* bump optional dependencies to 0.9.3 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.9.2...rari@0.9.3
## [rari@0.9.2] - 2026-02-14

### 🚀 Features

- *(rsc)* fix 404 not-found route handling with streaming support by @skiniks

### 🚜 Refactor

- *(rsc)* simplify not-found rendering and add cache key tests by @skiniks
- *(rsc)* extract streaming setup and add wire format validation by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump version to 0.9.2 by @skiniks
- *(rari)* bump optional dependencies to 0.9.2 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.9.1...rari@0.9.2
## [rari@0.9.1] - 2026-02-13

### 🚀 Features

- *(rari)* implement fetch caching with request deduplication by @skiniks
- *(rari)* enhance fetch caching with response headers and status text by @skiniks
- *(rari)* refactor fetch caching with header deduplication and content-type detection by @skiniks
- *(rari)* enhance fetch response handling with header extraction and cache key optimization by @skiniks

### 🚜 Refactor

- *(server)* extract client IP detection and make rate limiting environment-aware by @skiniks
- *(server)* implement custom IP extractor and add memory caps to rate limiters by @skiniks
- *(server)* replace HashMap with LRU cache for IP rate limiting by @skiniks
- *(server)* enhance rate limit logging and optimize cleanup by @skiniks
- *(server)* simplify rate limit logger with extracted IP utility by @skiniks
- *(server)* extract HTTP client to dedicated module by @skiniks
- *(runtime)* extract header conversion to dedicated function by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump version to 0.9.1 by @skiniks
- *(rari)* bump optional dependencies to 0.9.1 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.9.0...rari@0.9.1
## [rari@0.9.0] - 2026-02-12

### 🚀 Features

- *(runtime)* split client components from main bundle by @skiniks
- *(vite)* add import graph tracking for client-only component detection by @skiniks

### 🐛 Bug Fixes

- *(runtime)* improve error handling in file operations and component loading by @skiniks

### 🚜 Refactor

- *(rsc_renderer)* remove render_element_to_html module by @skiniks
- *(runtime)* extract component loading logic and improve client component resolution by @skiniks
- *(runtime)* unify component loading promise handling by @skiniks
- *(runtime)* improve component loading and path normalization by @skiniks

### 🎨 Styling

- format conditional statements and update linting rules by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.14...rari@0.9.0
## [rari@0.8.14] - 2026-02-05

### ⚙️ Miscellaneous Tasks

- *(rari)* add RariResponse to tsdown exports by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.13...rari@0.8.14
## [rari@0.8.13] - 2026-02-05

### 🚜 Refactor

- *(rsc)* migrate HTML sanitization to Rust ops by @skiniks
- *(rsc)* improve HTML rendering and sanitization logic by @skiniks
- *(rsc)* simplify root row selection logic in HTML rendering by @skiniks
- *(rsc)* extract style object serialization into reusable function by @skiniks
- *(rsc)* add unsigned integer handling in JSON rendering by @skiniks
- *(rsc)* improve root row selection logic in HTML rendering by @skiniks
- *(rsc)* expand self-closing HTML tags list for complete coverage by @skiniks
- *(rsc)* migrate route rendering from JavaScript to Rust by @skiniks
- *(rsc)* refactor HTML rendering with row caching and async boxed futures by @skiniks
- *(rsc)* reorder numeric type checks in style serialization by @skiniks
- *(rsc)* improve suspense symbol detection in HTML rendering by @skiniks
- *(rsc)* improve HTML attribute escaping and boolean handling by @skiniks
- *(rsc)* extract self-closing tags constant and improve style serialization by @skiniks
- *(rsc)* add tag name validation in HTML rendering by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump version to 0.8.12 by @skiniks
- *(rari)* bump optional dependencies to 0.8.12 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.12...rari@0.8.13
## [rari@0.8.12] - 2026-02-04

### 🐛 Bug Fixes

- *(proxy)* improve module path resolution and runtime imports by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.11...rari@0.8.12
## [0.8.10] - 2026-02-04

### 🚀 Features

- *(runtime)* export RSC client runtime modules and migrate to TypeScript by @skiniks
- *(vite)* add wildcard export support and rewrite runtime imports by @skiniks
- *(runtime)* add react-dom types and migrate to virtual RSC client module by @skiniks

### 🐛 Bug Fixes

- improve error handling and request header management by @skiniks
- *(runtime)* improve HTML escaping and RSC element parsing by @skiniks
- *(runtime)* remove unnecessary non-null assertions and improve line splitting by @skiniks
- *(vite)* add trailing comma to import replacement regex by @skiniks
- *(vite)* normalize virtual module imports with file extensions by @skiniks
- *(runtime)* improve error message handling in app rendering by @skiniks
- *(runtime)* improve attribute handling and import path resolution by @skiniks
- *(vite)* simplify import rewriting and improve virtual module resolution by @skiniks
- *(vite)* improve import rewriting with comprehensive pattern matching by @skiniks
- *(vite)* add runtime directory resolution for chunk imports by @skiniks
- *(vite)* improve error handling and add security validation for file resolution by @skiniks
- *(vite)* improve import path matching to support optional parent directory references by @skiniks
- *(vite)* add rari package detection for .mjs file resolution by @skiniks
- *(vite)* improve import path matching for react-server-dom-rari-client by @skiniks
- *(router)* move sitemap directory creation to after module validation by @skiniks
- *(vite)* skip TypeScript declaration files during directory scan by @skiniks

### 🚜 Refactor

- extract logger utilities into shared package by @skiniks
- improve cross-platform compatibility and code clarity by @skiniks
- simplify code and improve test mocking patterns by @skiniks
- improve type safety and path normalization by @skiniks
- *(proxy)* extract path normalization logic and add csrf retry test by @skiniks
- *(runtime)* replace global type declarations with accessor functions by @skiniks
- *(runtime)* extract getClientComponent to shared utility by @skiniks
- *(runtime)* extract global type definitions to shared types module by @skiniks
- *(runtime)* remove unused client component registry and utilities by @skiniks

### ⚙️ Miscellaneous Tasks

- *(runtime)* remove react-server-dom-shim type definitions by @skiniks
- *(package)* remove src directory from published files by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.10...v0.8.10
## [rari@0.8.10] - 2026-02-03

### 🚀 Features

- *(rsc-renderer)* improve error handling and RSC serialization format by @skiniks

### 🐛 Bug Fixes

- *(rsc-renderer)* simplify React element type checking logic by @skiniks
- *(vite)* remove redundant component path normalization by @skiniks
- *(rsc-renderer)* improve RSC payload prefix matching logic by @skiniks

### 🚜 Refactor

- *(rsc-renderer)* optimize RSC rendering and improve path handling by @skiniks
- *(rsc-renderer)* simplify style value string conversion logic by @skiniks
- *(rsc-renderer)* improve RSC payload detection and style value handling by @skiniks

### 🧪 Testing

- add comprehensive unit test suite with vitest by @skiniks

### ⚙️ Miscellaneous Tasks

- *(packages)* remove @rari/colors package and migrate to native Node.js utilities by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.9...rari@0.8.10
## [rari@0.8.9] - 2026-02-02

### 🚀 Features

- *(module-loader)* improve package resolution with workspace support by @skiniks
- *(module-loader)* improve workspace package resolution with nested container support by @skiniks

### 🚜 Refactor

- *(module-loader)* simplify workspace root detection logic by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.8...rari@0.8.9
## [rari@0.8.8] - 2026-02-02

### 🐛 Bug Fixes

- *(vite)* handle absolute paths in client and server component resolution by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.7...rari@0.8.8
## [rari@0.8.7] - 2026-02-02

### 🐛 Bug Fixes

- *(module_loader)* improve CommonJS module resolution and require handling by @skiniks
- *(module_loader)* improve CommonJS path resolution and error handling by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.6...rari@0.8.7
## [rari@0.8.6] - 2026-02-02

### 🚀 Features

- *(module_loader)* add CommonJS detection and ESM wrapping support by @skiniks
- *(module_loader)* improve CommonJS detection and require handling by @skiniks

### 🐛 Bug Fixes

- improve error handling and module type detection across build pipeline by @skiniks
- *(vite)* improve export parsing and rolldown output handling by @skiniks
- *(router)* disable code splitting in robots generator build config by @skiniks
- *(vite)* remove unnecessary resolveDir and external config from server build by @skiniks
- *(router)* disable file writing in build configs for generators by @skiniks
- *(module_loader)* improve error handling in CommonJS detection regex by @skiniks
- *(module_loader,router)* improve path resolution and build output handling by @skiniks
- *(router)* ensure output directory exists and improve robots file extension handling by @skiniks
- *(router)* improve type safety and module type handling in robots generator by @skiniks

### 🚜 Refactor

- *(router)* improve variable naming in robots generator by @skiniks

### ⚙️ Miscellaneous Tasks

- migrate remaining uses of esbuild to rolldown by @skiniks
- *(rari)* remove redundant no-op log message by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.5...rari@0.8.6
## [rari@0.8.5] - 2026-02-01

### 🚜 Refactor

- *(rari)* simplify tsdown configuration and external dependencies by @skiniks
- *(packages)* standardize tsconfig includes and improve deploy exports by @skiniks
- *(cache)* move cache directory to .cache and improve initialization by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump version to 0.8.5 by @skiniks
- *(rari)* bump binary package versions to 0.8.5 by @skiniks
- *(rari)* bump version to 0.8.4 by @skiniks
- *(rari)* revert version to 0.8.4 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.4...rari@0.8.5
## [0.8.4] - 2026-01-30

### 🐛 Bug Fixes

- correct typos and improve error handling in RSC rendering by @skiniks
- correct error handling and redirect URL assignment by @skiniks

### 🚜 Refactor

- extract React component and SSR manifest creation logic by @skiniks
- *(rsc)* remove unused rowId parameter from render functions by @skiniks
- *(vite)* extract error messages to variables for clarity by @skiniks
- *(vite)* remove unused component tracking sets by @skiniks

### 🎨 Styling

- add blank lines for improved code readability by @skiniks
- remove unnecessary braces from single-statement conditionals by @skiniks
- simplify conditional expressions and improve code readability by @skiniks
- improve regex patterns for string and path normalization by @skiniks
- *(rsc)* remove redundant loadingComponent check by @skiniks
- *(metadata_collector)* remove unused eslint directive and simplify error handling by @skiniks
- *(rsc)* remove unused isAsyncResult assignment by @skiniks
- remove unused variables and simplify property deletion by @skiniks
- *(render_script)* simplify lazy children detection logic by @skiniks
- remove unused variable assignments and simplify encoding defaults by @skiniks
- *(cli)* remove unnecessary blank lines in detectPackageManager by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.3...v0.8.4
## [0.8.3] - 2026-01-30

### 🚀 Features

- *(image-optimizer)* add remote URL validation for security by @skiniks
- *(image-optimizer)* add request timeouts and improve URL validation by @skiniks

### 🐛 Bug Fixes

- *(server)* add URL validation and manual redirect handling by @skiniks
- *(image-optimizer)* redact sensitive URL data in debug logs by @skiniks
- *(image-optimizer)* enhance remote URL validation with private IP detection by @skiniks
- *(image-optimizer)* improve private IP range detection and validation by @skiniks
- *(image-optimizer)* enhance IPv6 private address detection by @skiniks
- *(runtime)* redact RSC operation JSON in error logs by @skiniks

### 🚜 Refactor

- *(runtime)* improve HTTP client error handling with Result type by @skiniks

### ⚡ Performance

- *(image-optimizer)* pre-allocate vector capacity for response bytes by @skiniks
- *(runtime)* implement HTTP client singleton with OnceLock by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.2...v0.8.3
## [rari@0.8.2] - 2026-01-30

### ⚙️ Miscellaneous Tasks

- *(rari)* bump optional dependencies to 0.8.2 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.8.2...rari@0.8.2
## [0.8.2] - 2026-01-29

### 🚀 Features

- *(image)* add quality allowlist configuration for image optimization by @skiniks
- *(image)* add preoptimization manifest support for image variants by @skiniks
- *(image)* add preload image tracking and metadata injection support by @skiniks
- *(image)* improve image scanner and optimizer robustness by @skiniks
- *(image)* improve image scanner and optimizer robustness by @skiniks

### 🐛 Bug Fixes

- *(image)* move preload registration after dry-run check by @skiniks

### 🚜 Refactor

- *(react)* remove polyfill for Object.hasOwn by @skiniks
- *(metadata-injection)* simplify nested conditionals with let-else chains by @skiniks
- *(metadata-injection)* simplify image optimizer reference handling by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.1...v0.8.2
## [rari@0.8.1] - 2026-01-29

### 🚀 Features

- *(cli)* add automatic package manager detection and cross-platform execution by @skiniks
- *(cli)* improve package manager detection with monorepo support by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump optional dependencies to 0.8.1 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.8.1...rari@0.8.1
## [0.8.1] - 2026-01-29

### 🚀 Features

- *(vite)* expand optimizeDeps configuration for React core modules by @skiniks
- *(image)* add CLI subcommand for pre-optimizing local images by @skiniks
- *(rari)* export Metadata type and add type annotations to metadata exports by @skiniks
- *(image)* add dry-run mode to image optimization CLI and improve type safety by @skiniks
- *(image)* add rkyv serialization for image cache and improve async file operations by @skiniks

### 🚜 Refactor

- *(rari)* improve path normalization and proxy configuration by @skiniks
- *(rari)* separate server config from manifest and rename types by @skiniks
- improve code quality and type safety across codebase by @skiniks
- improve async handling and optimize image processing pipeline by @skiniks
- modernize code patterns and improve image optimization configuration by @skiniks
- optimize image processing and improve code constants by @skiniks
- improve code clarity and boolean logic in image optimizer and sidebar by @skiniks
- *(image)* optimize cache operations and improve async file handling by @skiniks
- *(image)* make cache get method async and improve file I/O handling by @skiniks
- *(image)* improve dry-run logging format by @skiniks
- *(image)* convert local file read to async operation by @skiniks
- *(image)* improve public directory existence check with async operation by @skiniks
- migrate to ES modules and improve cross-platform compatibility by @skiniks
- *(image)* add path traversal protection for local file access by @skiniks
- *(logger)* extract logging utilities into dedicated module by @skiniks
- *(logger)* remove deprecated logWarning function in favor of logWarn by @skiniks

### 🎨 Styling

- modernize JavaScript and TypeScript code patterns by @skiniks
- remove unnecessary braces from single-statement conditionals by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* remove unused lazy promise resolution script by @skiniks
- upgrade Node.js minimum version to 20.6.0 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.8.0...v0.8.1
## [rari@0.8.0] - 2026-01-27

### ⚙️ Miscellaneous Tasks

- *(rari)* update optional dependencies to 0.8.0 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.8.0...rari@0.8.0
## [0.8.0] - 2026-01-27

### 🚀 Features

- *(csp)* add worker-src directive support by @skiniks
- *(vite)* add HTML import detection and build optimization by @skiniks
- *(sitemap)* add dynamic sitemap generation support by @skiniks
- *(sentry)* implement dynamic import and optimize bundle splitting by @skiniks

### 🐛 Bug Fixes

- *(router)* simplify server URL resolution in ClientRouter by @skiniks
- *(rsc)* unwrap React.Fragment layout components correctly by @skiniks

### 🚜 Refactor

- *(colors)* extract colors utility into standalone package by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.7.14...v0.8.0
## [rari@0.7.14] - 2026-01-27

### 🐛 Bug Fixes

- *(vite)* correct client reference property names by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.7.13...rari@0.7.14
## [rari@0.7.13] - 2026-01-27

### 🚀 Features

- *(vite)* improve server build module resolution and client component handling by @skiniks

### 🚜 Refactor

- *(vite)* rename server build plugin for clarity by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.7.12...rari@0.7.13
## [0.7.9] - 2026-01-27

### 🚀 Features

- *(vite)* enhance client component scanning and import resolution by @skiniks
- *(vite)* enhance server-side component and action reference handling by @skiniks

### 🐛 Bug Fixes

- *(vite)* update component import regex to support alias paths by @skiniks

### 💼 Other

- Add DoS mitigations to server action validation by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.7.11...v0.7.9
## [rari@0.7.11] - 2026-01-26

### 🚀 Features

- *(rari)* add platform entry point to tsdown by @skiniks
- *(rari)* add input options to suppress postcss export warnings by @skiniks

### 🚜 Refactor

- *(router)* replace chokidar with vite dev server watcher by @skiniks
- *(router)* replace dynamic import function with vite-ignore comments by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.7.10...rari@0.7.11
## [rari@0.7.10] - 2026-01-24

### 💼 Other

- *(tsdown)* enable minification for build output by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.7.9...rari@0.7.10
## [rari@0.7.9] - 2026-01-24

### 🚜 Refactor

- *(vite)* replace acorn with esbuild by @skiniks
- *(rari)* replace picocolors with internal colors util by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.7.8...rari@0.7.9
## [0.7.7] - 2026-01-23

### 🚀 Features

- *(rendering)* fix support for dangerouslySetInnerHTML and object-style props by @skiniks
- *(cli)* add dev command and support scoped package names by @skiniks
- *(cli)* add build command and unify package scripts by @skiniks

### 🐛 Bug Fixes

- Suspense boundary support with streaming by @skiniks

### 🚜 Refactor

- *(og-cache)* defer cache directory creation until first use by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.7.6...v0.7.7
## [0.7.6] - 2026-01-21

### 🐛 Bug Fixes

- *(rsc)* update eslint and oxlint rules in page render script by @skiniks

### 🚜 Refactor

- *(exports)* reorganize proxy and image module exports by @skiniks

### 📚 Documentation

- *(rari)* standardize rari branding to lowercase in package description by @skiniks

### 🎨 Styling

- standardize rari branding to lowercase in console messages by @skiniks
- standardize rari branding to lowercase in error messages by @skiniks
- standardize rari branding to lowercase in runtime logs by @skiniks
- standardize rari branding in log messages and error outputs by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump optional dependencies to 0.7.5 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.7.5...v0.7.6
## [0.7.5] - 2026-01-18

### 🚀 Features

- *(rari)* refactor routing and rendering architecture with improved component resolution by @skiniks
- *(rari,web)* add custom define option and integrate sponsor link into navigation by @skiniks
- *(og)* add ImageResponse documentation by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.7.4...v0.7.5
## [rari@0.7.4] - 2026-01-15

### ⚙️ Miscellaneous Tasks

- *(rari)* bump binary version to 0.7.4 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.7.4...rari@0.7.4
## [0.7.4] - 2026-01-15

### 🚀 Features

- *(rari)* optimize component resolution and rendering performance by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump binary version to 0.7.4 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.7.3...v0.7.4
## [rari@0.7.3] - 2026-01-14

### ⚙️ Miscellaneous Tasks

- *(rari)* bump binary version to 0.7.3 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.7.3...rari@0.7.3
## [0.7.3] - 2026-01-14

### 🚀 Features

- *(rari)* enhance open graph and twitter metadata handling by @skiniks
- *(rari)* add body scripts extraction and injection support by @skiniks

### 🚜 Refactor

- rename docs directory to web and update references by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* bump binary version to 0.7.3 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.7.2...v0.7.3
## [rari@0.7.1] - 2026-01-13

### ⚙️ Miscellaneous Tasks

- *(rari)* bump optional dependencies to 0.7.1 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.7.1...rari@0.7.1
## [0.7.1] - 2026-01-13

### ⚙️ Miscellaneous Tasks

- remove dav1d dependency and update dependencies by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.7.0...v0.7.1
## [rari@0.7.0] - 2026-01-13

### 🚀 Features

- *(image)* add image optimization and caching system by @skiniks
- *(og)* add open graph image generation with dynamic rendering by @skiniks
- *(security)* add IP-based rate limiting for critical endpoints by @skiniks
- *(image)* add local image pattern matching and AVIF native support by @skiniks
- *(og)* inject generated OG images into page metadata by @skiniks
- *(security)* add environment-aware rate limiting configuration by @skiniks
- *(image)* change default image format from WebP to AVIF by @skiniks
- *(security)* make CSRF protection optional with environment configuration by @skiniks

### 🐛 Bug Fixes

- *(rsc)* handle stale content and missing promises gracefully by @skiniks

### 🚜 Refactor

- *(error)* simplify error enum and clippy configuration by @skiniks
- *(hmr)* consolidate HMR handlers into unified action endpoint by @skiniks
- *(server)* move register endpoints to development-only routes by @skiniks
- *(rsc)* stream RSC responses directly without buffering by @skiniks
- *(sync)* replace std::sync with parking_lot for better performance by @skiniks
- *(vite)* fix client component HMR handling by @skiniks
- *(image)* rename priority to preload and add custom loader support by @skiniks
- *(api)* consolidate internal routes under /_rari namespace by @skiniks
- *(path)* remove unused custom paths constructor and test by @skiniks
- *(layout)* remove unused skeleton validation and layout structure methods by @skiniks
- *(spam_blocker)* add block reason tracking and improve response handling by @skiniks
- *(server)* improve app router error handling and validation by @skiniks
- *(cache)* remove production environment check from caching logic by @skiniks
- *(rendering)* improve HTML structure handling and asset extraction by @skiniks
- *(config)* migrate CSP and rate limit config from environment variables to manifest by @skiniks
- *(config)* remove TOML file support and simplify configuration loading by @skiniks

### ⚡ Performance

- *(rsc)* replace string allocations with cow-utils for efficiency by @skiniks

### 🎨 Styling

- *(rsc)* remove unnecessary braces from single-statement conditionals by @skiniks
- *(node)* fix formatting in promisify callback handler by @skiniks

### ⚙️ Miscellaneous Tasks

- *(router)* remove item from skip directories list by @skiniks
- *(rari)* bump optional dependencies to 0.7.0 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.6.1...rari@0.7.0
## [0.6.1] - 2026-01-08

### 🚀 Features

- *(server)* implement separate rate limiting for revalidation endpoints by @skiniks
- *(server)* expand spam blocker patterns for WordPress and IDE configs by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.6.0...v0.6.1
## [0.6.0] - 2026-01-08

### 🚀 Features

- *(proxy)* add request/response proxy middleware and runtime execution by @skiniks
- *(middleware)* add spam blocker middleware for request filtering by @skiniks
- *(robots)* add robots.txt generation support by @skiniks
- *(metadata)* add comprehensive metadata support for icons, theme, and apple web app by @skiniks
- *(vite)* skip robots and sitemap files in server component scanning by @skiniks

### 🚜 Refactor

- *(router)* remove loading component map generation by @skiniks

### 🎨 Styling

- *(router)* remove unnecessary braces from conditional statements by @skiniks
- *(rari)* remove unnecessary braces from conditional statements by @skiniks
- *(router)* remove unnecessary braces from conditional statements by @skiniks
- remove unnecessary braces from conditional statements by @skiniks
- remove unnecessary braces from conditional statements by @skiniks
- *(vite)* remove unnecessary braces from conditional statements by @skiniks
- *(rari)* remove unnecessary braces from conditional statements by @skiniks
- *(rari)* remove unnecessary braces from conditional statements by @skiniks
- *(router)* remove unnecessary braces from conditional statements by @skiniks
- *(vite)* remove esbuildOptions deprecation warning suppression by @skiniks
- *(rari)* remove unnecessary blank lines from conditional statements by @skiniks
- *(rari)* remove unnecessary blank lines from vite plugin by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* remove useActionState hook and exports by @skiniks
- *(rari)* remove file extensions from mdx exports by @skiniks
- *(rari)* remove AppRouterProvider export from package.json by @skiniks
- *(rari)* remove fsevents from external dependencies by @skiniks
- *(vite)* remove external dependencies configuration by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.30...v0.6.0
## [rari@0.5.30] - 2026-01-03

### 🚀 Features

- *(rsc)* add client-side RSC fetching and dev server proxying by @skiniks

### 🚜 Refactor

- *(router)* remove unused exports and internal utilities by @skiniks
- *(router)* remove legacy layout and runtime management systems by @skiniks

### ⚙️ Miscellaneous Tasks

- *(router)* remove router index barrel export and consolidate exports by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.29...rari@0.5.30
## [0.5.23] - 2026-01-02

### 🚀 Features

- *(rsc)* improve streaming updates by @skiniks
- *(rsc)* enhance lazy loading and streaming completion handling by @skiniks
- *(rsc-client-runtime)* improve RSC row parsing by @skiniks
- *(rsc)* optimize lazy loading and promise resolution handling by @skiniks
- *(rari)* implement partial hydration and dynamic module loading by @skiniks
- *(rsc)* implement lazy promise resolution and streaming suspense by @skiniks
- *(rsc)* rename internal module markers from double underscore to tilde prefix by @skiniks
- *(rari)* add chunked transfer encoding to streaming RSC responses by @skiniks
- *(server)* add stream compression support with zstd, brotli, and gzip by @skiniks

### 🐛 Bug Fixes

- *(rsc)* correct row ID generation in serializer by @skiniks
- *(rari)* remove debug console.warn statements from RSC client by @skiniks

### 🚜 Refactor

- *(rsc)* extract HTML closing generation into dedicated method by @skiniks
- *(server)* remove x-accel-buffering header from streaming responses by @skiniks
- *(rsc)* remove suspense module and consolidate functionality by @skiniks

### 🎨 Styling

- *(rari)* remove unnecessary comment by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.28...v0.5.23
## [0.5.22] - 2025-12-24

### 🚀 Features

- *(server)* invalidate caches on server action redirects by @skiniks

### 🐛 Bug Fixes

- *(server)* return correct HTTP status codes for not-found routes by @skiniks

### 🚜 Refactor

- *(module-loader)* improve dynamic import path resolution logic by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.27...v0.5.22
## [0.5.21] - 2025-12-24

### 🚀 Features

- *(server)* implement on-demand revalidation and response caching by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.26...v0.5.21
## [0.5.20] - 2025-12-23

### 🚀 Features

- *(routing)* implement dynamic route info endpoint and remove manifest injection by @skiniks

### 🐛 Bug Fixes

- *(server)* block access to app-routes.json in request handlers by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.25...v0.5.20
## [0.5.19] - 2025-12-23

### 🚀 Features

- *(rsc)* add RSC wire format payload and manifest embedding by @skiniks
- *(mdx)* replace mdx-remote with native @mdx-js/mdx compilation by @skiniks

### 🐛 Bug Fixes

- *(rsc)* correct React element symbol and add transitional element support by @skiniks
- *(rsc)* correct React symbol property names from single to double dollar signs by @skiniks
- *(rsc)* update React element symbol to use transitional variant by @skiniks
- *(rsc)* remove ref property from React element objects by @skiniks
- *(rsc)* remove ref property from React element objects by @skiniks
- *(rsc)* update React element symbol to use transitional variant by @skiniks
- *(rsc)* remove ref property and forwardRef from React elements by @skiniks

### 🎨 Styling

- *(rsc)* remove unnecessary braces from single-line conditional by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.24...v0.5.19
## [0.5.18] - 2025-12-20

### 🚜 Refactor

- *(rsc)* rename client component registry globals to use tilde prefix by @skiniks
- *(rsc)* migrate global namespace from __rari to ~rari by @skiniks
- *(rsc)* migrate global namespace from __rari to ~rari by @skiniks
- *(rsc)* migrate global namespace from __rsc to ~rsc by @skiniks
- *(rsc)* remove Counter component special handling from RSC traversal by @skiniks
- *(rsc)* migrate global namespace from double underscore to tilde prefix by @skiniks
- *(rsc)* migrate global namespace from double underscore to tilde prefix by @skiniks
- *(rsc)* migrate global namespace from double underscore to tilde prefix by @skiniks
- *(runtime)* migrate error handling globals from double underscore to tilde prefix by @skiniks
- *(rsc)* migrate registry proxy global from double underscore to tilde prefix by @skiniks
- *(hmr)* improve component specifier handling and remove debug logging by @skiniks
- *(rsc)* remove debug logging statements across codebase by @skiniks
- *(runtime)* remove lifecycle logging from LayoutWrapper by @skiniks

### ⚙️ Miscellaneous Tasks

- *(logging)* remove info-level logging statements across codebase by @skiniks
- *(logging)* remove warn-level logging statements by @skiniks
- *(logging)* remove warn-level logging and upgrade to error-level where appropriate by @skiniks
- *(logging)* remove warn-level logging and unused variables by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.23...v0.5.18
## [rari@0.5.23] - 2025-12-18

### ⚙️ Miscellaneous Tasks

- *(rari)* bump optional dependencies to 0.5.17 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.5.17...rari@0.5.23
## [0.5.17] - 2025-12-18

### 🚀 Features

- *(rsc)* implement getData-based page not found detection by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.22...v0.5.17
## [rari@0.5.22] - 2025-12-17

### ⚙️ Miscellaneous Tasks

- *(rari)* bump optional dependencies to 0.5.16 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.5.16...rari@0.5.22
## [0.5.16] - 2025-12-17

### 🐛 Bug Fixes

- *(server)* return 404 status code for not found routes by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.21...v0.5.16
## [rari@0.5.21] - 2025-12-17

### ⚙️ Miscellaneous Tasks

- *(rari)* bump optional dependencies to 0.5.15 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.5.15...rari@0.5.21
## [0.5.15] - 2025-12-17

### 🐛 Bug Fixes

- *(server)* block access to sensitive internal files by @skiniks
- *(server)* block access to sensitive internal files by @skiniks

### 🚜 Refactor

- *(server)* separate stylesheet and script injection into head and body by @skiniks
- *(rsc)* standardize boundary ID prop naming to ~boundaryId by @skiniks
- *(rsc)* standardize pre-serialized suspense prop naming by @skiniks
- *(rsc)* standardize client component marker prop naming by @skiniks
- *(rsc)* standardize data attribute naming with tilde prefix by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.20...v0.5.15
## [0.5.14] - 2025-12-17

### 🚀 Features

- *(runtime)* add module loading and component initialization improvements by @skiniks
- *(rari)* add external module configuration for server builds by @skiniks
- *(server)* improve server action module loading with ESM support by @skiniks

### 🚜 Refactor

- *(vite)* remove unnecessary banners and optimize minification settings by @skiniks
- *(rsc)* remove legacy module fallback patterns and simplify runtime by @skiniks
- *(vite)* simplify node imports transformation in server build by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.19...v0.5.14
## [0.5.12] - 2025-12-16

### 🐛 Bug Fixes

- *(server)* correct asset injection detection logic by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.17...v0.5.12
## [0.5.11] - 2025-12-16

### 🚀 Features

- *(metadata)* add page metadata collection and injection system by @skiniks

### 🚜 Refactor

- *(server)* extract html wrapping logic into function by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.16...v0.5.11
## [rari@0.5.16] - 2025-12-13

### 🚜 Refactor

- *(docs,examples)* migrate to path aliases by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.15...rari@0.5.16
## [0.5.10] - 2025-12-12

### 🚀 Features

- *(rsc)* enhance prop serialization and client component registration by @skiniks
- *(runtime)* enhance Node.js compatibility stubs for fs, path, and process by @skiniks

### 🐛 Bug Fixes

- *(router)* improve component loading fallback logic by @skiniks

### 🚜 Refactor

- *(docs,rari)* improve markdown rendering and module resolution by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.14...v0.5.10
## [0.5.8] - 2025-12-11

### 🐛 Bug Fixes

- add Win32_System_SystemInformation feature for windows-sys by @skiniks
- *(deps)* expand windows-sys features for Windows API support by @skiniks
- *(deps)* add Win32_System_SystemInformation feature to windows-sys by @skiniks
- *(deps)* add Win32_Networking_WinSock feature to windows-sys by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.5.7...v0.5.8
## [0.5.7] - 2025-12-11

### 🐛 Bug Fixes

- *(cli)* guard main execution to prevent running when imported as module by @skiniks

### 🚜 Refactor

- *(runtime)* remove unnecessary clone on cache options by @skiniks

### ⚙️ Miscellaneous Tasks

- *(server)* simplify startup logging and remove verbose debug output by @skiniks
- *(rari)* disable doctests and remove bin tests by @skiniks
- *(server)* remove verbose startup completion messages by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.12...v0.5.7
## [rari@0.5.12] - 2025-12-11

### 🚀 Features

- *(router)* improve loading component handling and validation by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.11...rari@0.5.12
## [rari@0.5.11] - 2025-12-11

### 🚀 Features

- *(rari)* improve client-server routing and add manifest middleware by @skiniks
- *(router)* convert loading component modules to Map by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.10...rari@0.5.11
## [rari@0.5.10] - 2025-12-09

### ⚙️ Miscellaneous Tasks

- *(rari)* bump optional dependencies to 0.5.6 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.5.6...rari@0.5.10
## [0.5.6] - 2025-12-09

### 🚀 Features

- *(server)* add path validation utilities for security by @skiniks
- *(server)* add path validation to HMR component reload handler by @skiniks
- *(server)* add CSRF token protection to server actions by @skiniks
- *(server)* add CSRF token generation and injection to HTML responses by @skiniks
- *(server)* integrate CSRF token generation into RSC rendering pipeline by @skiniks
- *(server)* add configurable CORS support with origin validation by @skiniks
- *(server)* add input validation with configurable depth and size limits by @skiniks
- *(server)* add redirect URL validation with configurable host allowlist by @skiniks
- *(server)* add configurable Content Security Policy support by @skiniks
- *(server)* enhance security headers and simplify CSP configuration by @skiniks
- *(server)* add granular body size limits for API routes by @skiniks
- *(error)* add HTTP status codes and safe error messages for API responses by @skiniks
- *(server)* add configurable rate limiting with per-IP tracking by @skiniks
- *(csrf)* move CSRF token generation to client-side runtime by @skiniks
- *(actions)* add reserved export name validation for server actions by @skiniks
- *(rsc)* add special value and reference type parsing for wire format by @skiniks

### ⚙️ Miscellaneous Tasks

- upgrade dependencies and enhance linting rules by @skiniks
- bump rari version to 0.5.6 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.9...v0.5.6
## [rari@0.5.9] - 2025-12-09

### 🚀 Features

- *(rsc)* extract RSC wire format escaping utilities by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.8...rari@0.5.9
## [0.5.4] - 2025-12-04

### 🚀 Features

- *(rsc)* extract client-side RSC runtime from inline and update rendering pipeline by @skiniks
- *(runtime)* extract client-side entry point and RSC wire format parser by @skiniks

### 🚜 Refactor

- *(rsc)* reorganize module structure for rendering pipeline by @skiniks
- *(runtime)* use StreamOpState from ops module by @skiniks
- *(server)* reorganize vite proxy into module structure by @skiniks
- *(server)* reorganize types by @skiniks
- *(server)* reorganize module structure by @skiniks
- *(runtime)* move transpile module into utils by @skiniks
- *(rsc)* extract JavaScript initialization and rendering scripts by @skiniks

### 🎨 Styling

- *(runtime)* clean up linting directives and modernize string formatting by @skiniks

### ⚙️ Miscellaneous Tasks

- *(rari)* configure cargo-machete to ignore num-bigint-dig dependency by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.7...v0.5.4
## [0.5.2] - 2025-12-03

### 🐛 Bug Fixes

- *(router)* Add Vite ignore comments to dynamic imports by @skiniks
- *(security)* prevent prototype pollution in server actions (CVE-2025-55182) by @skiniks

### 🚜 Refactor

- *(rsc)* Extract and reorganize type definitions and fix dupes by @skiniks
- *(server)* Reorganize server module structure by @skiniks
- *(runtime)* Extract runtime_factory into submodules by @skiniks
- *(module_loader)* Extract module loader into submodules by @skiniks
- *(runtime)* Extract runtime submodules into new org by @skiniks
- *(rsc)* Reorganize RSC module structure into submodules by @skiniks
- *(rsc)* Extract streaming module scripts into separate files by @skiniks
- *(rsc)* Extract streaming submodules into separate files by @skiniks
- *(rsc)* Extract renderer scripts and constants into separate files by @skiniks
- *(rsc)* Extract renderer core logic and utilities into submodules by @skiniks
- *(rsc)* Extract layout renderer scripts into separate files by @skiniks
- *(layout_renderer)* Extract core logic and utilities into submodules by @skiniks
- *(rsc)* Remove unused fields and dead code by @skiniks
- *(runtime)* Remove unused fields and dead code by @skiniks
- *(rsc,runtime)* Remove auto-register logic and hardcoded paths by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.5...v0.5.2
## [0.5.1] - 2025-12-02

### 🚜 Refactor

- *(rsc)* Replace timestamp-based key generation with counter for stable key gen by @skiniks

### ⚙️ Miscellaneous Tasks

- bump binary packages to v0.5.1 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.4...v0.5.1
## [rari@0.5.4] - 2025-11-24

### 🚜 Refactor

- *(cli)* fix for issue #45 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.5.3...rari@0.5.4
## [0.5.0] - 2025-11-24

### 🚀 Features

- *(error)* Add comprehensive streaming and loading state error handling by @skiniks
- *(rsc)* Add RSC wire format parser and streaming layout renderer by @skiniks
- *(rsc)* Refactor streaming render completion to use async/await with channel signaling by @skiniks
- *(router)* Implement client-side routing with layout management and error handling by @skiniks

### 🚜 Refactor

- *(vite)* Simplify client-side rendering logic by @skiniks
- *(exports)* Rename server export to vite for clarity by @skiniks

### 🧪 Testing

- *(layout_renderer)* Remove obsolete assertion messages from tests by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.4.0...v0.5.0
## [rari@0.4.0] - 2025-10-30

### 🚀 Features

- *(runtime)* Add API request bridge for Rari server runtime by @skiniks
- *(runtime)* Upgrade Deno dependencies and improve module loading by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.3.3...rari@0.4.0
## [rari@0.3.3] - 2025-10-28

### 🚜 Refactor

- *(rsc)* Remove JSX transformation from core  and simplify dependency extraction by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.3.2...rari@0.3.3
## [0.3.2] - 2025-10-28

### 🐛 Bug Fixes

- prevent false ES module detection when code contains "export" in strings by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.3.1...v0.3.2
## [0.3.1] - 2025-10-24

### 🚀 Features

- *(rari)* Add pathname support for layouts and server components by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.3.0...v0.3.1
## [0.3.0] - 2025-10-21

### 🚀 Features

- implement app router with dynamic routing, layouts, and error handling; add example application by @skiniks
- add static asset serving and production handling in server by @skiniks
- implement server actions and enhance form handling with progressive enhancement in example app by @skiniks
- *(HMR)* implement component reload functionality and error handling by @skiniks
- *(ssr)* introduce server-side rendering capabilities and configuration options by @skiniks
- *(ssr)* enhance SSR rendering with raw content handling and improved child rendering logic by @skiniks
- *(ssr)* Enhance script and link tag extraction with improved handling by @skiniks
- *(rsc)* Enhance layout rendering with performance metrics and direct HTML rendering by @skiniks
- *(rsc)* introduce direct HTML rendering and enhance layout rendering with caching by @skiniks
- *(runtime)* Enhance request context handling and runtime integration by @skiniks
- *(rsc)* introduce RSC HTML rendering and refactor SSR components by @skiniks
- *(router)* Add support for global and scoped not-found routes by @skiniks

### 🐛 Bug Fixes

- correct component path replacement by @skiniks

### 🚜 Refactor

- remove 'use server' directives from components and update documentation to clarify server/client component usage by @skiniks
- replace entry-client.tsx with virtual import for client components by @skiniks
- simplify server component detection logic by removing redundant checks by @skiniks
- streamline rari router plugin by removing unused options and simplifying route generation logic by @skiniks
- clean up routing code by deleting obsolete files by @skiniks
- remove obsolete '.rari' directory references and streamline routing logic by @skiniks
- unify cache configuration types and simplify route matching logic in server module by @skiniks
- enhance middleware handling and dynamic Vite port config by @skiniks
- enhance async handling in layout and streaming renderers, update RSC traversal logic for better error handling by @skiniks
- remove unused test handler and direct registration route by @skiniks
- expanding HMR for app router support by @skiniks
- *(error)* Improve ModuleReloadError handling with boxed error variant by @skiniks
- *(AppRouterHMRProvider)* streamline event handling and enhance RSC payload management by @skiniks
- *(ssr)* reorganize SSR rendering logic and improve error handling by @skiniks
- *(rsc)* Remove renderer pool and simplify layout rendering by @skiniks
- *(rari)* simplify server component checks and remove unused directory validations by @skiniks
- *(rsc)* Simplify caching logic and remove route-specific cache exclusions by @skiniks
- *(rsc)* Rename RSC function registration global to improve clarity by @skiniks
- *(rsc)* Remove HTML diagnostics and simplify rendering logging by @skiniks
- *(rsc)* enhance HTML template handling and logging by @skiniks
- *(rsc)* consolidate HTML rendering logic and improve cache management by @skiniks
- *(rsc)* Simplify component registration and remove redundant hash-based keys by @skiniks
- *(rsc)* Rename RSC rendering functions to follow consistent naming convention by @skiniks
- *(rsc)* Improve component invalidation and registration mechanisms by @skiniks
- *(rsc)* Improve component import transformation for global component resolution by @skiniks
- *(rsc)* Enhance HTML rendering with raw content support and vendor prefix handling by @skiniks
- *(rsc)* Improve client-side rendering and hydration logic by @skiniks
- *(rsc)* fix hydration logic in renderApp by @skiniks
- *(server)* Enhance static file handling and security configuration by @skiniks
- *(server)* Enhance Content Security Policy configuration by @skiniks
- *(rsc)* Enhance layout renderer test cases with not_found support by @skiniks

### 📚 Documentation

- *(rari)* Add comprehensive package metadata for Cargo.toml by @skiniks

### ⚙️ Miscellaneous Tasks

- update server and Vite configuration ports to 5173 by @skiniks
- *(dependencies)* Update project dependencies to latest versions by @skiniks
- *(dependencies)* Update project dependencies by @skiniks
- fix linting errors by @skiniks
- *(dependencies)* Update project dependencies and linting config by @skiniks
- *(rari)* Bump package version to 0.3.0 by @skiniks
- *(rari)* Bump package version to 0.3.0 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.24...v0.3.0
## [rari@0.2.24] - 2025-09-23

### ⚙️ Miscellaneous Tasks

- update dependencies across multiple packages to latest versions by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.23...rari@0.2.24
## [rari@0.2.23] - 2025-09-23

### 🚀 Features

- add support for component initial loading state and enhance HMR functionality by @skiniks

### 🚜 Refactor

- enhance error handling and retry logic in component rendering and registration processes by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.22...rari@0.2.23
## [rari@0.2.22] - 2025-09-04

### 🚜 Refactor

- improve formatting and error handling in extractCacheConfigFromContent function by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.21...rari@0.2.22
## [rari@0.2.21] - 2025-09-04

### ⚙️ Miscellaneous Tasks

- update rari dependencies to version 0.2.15 and remove unused client-dev entry by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.2.15...rari@0.2.21
## [0.2.15] - 2025-09-04

### ⚙️ Miscellaneous Tasks

- update rari to version 0.2.8 and bump dependencies across multiple packages by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.20...v0.2.15
## [rari@0.2.20] - 2025-09-04

### 🚜 Refactor

- remove unused router components and functions, simplify package.json dependencies by @skiniks

### ⚙️ Miscellaneous Tasks

- update dependencies across multiple packages to latest versions by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.19...rari@0.2.20
## [rari@0.2.19] - 2025-09-02

### ⚙️ Miscellaneous Tasks

- update rari to version 0.2.7, add caching configuration support, and remove macOS Rust version override in release workflow by @skiniks
- update optionalDependencies for rari to version 0.2.14 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.18...rari@0.2.19
## [rari@0.2.18] - 2025-08-22

### ⚙️ Miscellaneous Tasks

- bump rari version to 0.2.6 and update type annotations by @skiniks
- update dependencies by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.17...rari@0.2.18
## [rari@0.2.17] - 2025-08-21

### ⚙️ Miscellaneous Tasks

- adjust resource limits and improve performance by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.16...rari@0.2.17
## [rari@0.2.16] - 2025-08-21

### ⚙️ Miscellaneous Tasks

- update rari crate version to 0.2.4, adjust dependencies, and improve error handling in module loader by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.15...rari@0.2.16
## [rari@0.2.15] - 2025-08-20

### 🐛 Bug Fixes

- preserve React module side effects to prevent import transformation issues by @skiniks
- rolling back aggressive optimizations by @skiniks

### ⚙️ Miscellaneous Tasks

- update optionalDependencies in package.json to version 0.2.2 for Rari binaries by @skiniks
- bump rari package version to 0.2.3 by @skiniks
- update rari package version to 0.2.4 and format Cargo.toml for consistency by @skiniks
- update rari package version to 0.2.13 and bump optional dependencies to 0.2.9 by @skiniks
- bump rari package version to 0.2.14 by @skiniks
- increase resource limits and configuration parameters for improved performance by @skiniks
- bump rari package version to 0.2.3 by @skiniks
- bump rari package version to 0.2.15-next.0 by @skiniks
- remove CHANGELOG.md and update release script to copy generated changelog to package directory by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.2.2...rari@0.2.15
## [0.2.2] - 2025-08-15

### ⚙️ Miscellaneous Tasks

- update rari package to use workspace link and remove unused cache control settings by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.2...v0.2.2
## [rari@0.2.2] - 2025-08-15

### 🚀 Features

- enhance documentation and meta description handling by @skiniks
- enhance caching and HTML optimization in Rari server by @skiniks
- enhance Google Tag Manager integration and improve layout responsiveness by @skiniks

### 💼 Other

- Framework-level React DOM optimizations - partial implementation by @skiniks

### 🚜 Refactor

- enhance router functionality and improve code structure by @skiniks
- streamline Vite configuration and enhance component registration by @skiniks

### ⚙️ Miscellaneous Tasks

- update rari package version to 0.2.8 in package.json and dependencies by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.8...rari@0.2.2
## [0.2.7] - 2025-08-16

### ⚙️ Miscellaneous Tasks

- bump all package versions to 0.2.7 after rollback to stable commit by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.1...v0.2.7
## [rari@0.2.1] - 2025-08-13

### 🚜 Refactor

- streamline RscClient endpoint configuration by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.2.0...rari@0.2.1
## [0.2.0] - 2025-08-13

### 🚀 Features

- enhance Vite configuration for React integration by @skiniks

### 🚜 Refactor

- enhance error handling and improve streaming functionality by @skiniks
- streamline error handling and reduce console warnings in init_react.js by @skiniks
- remove streaming_v2 module and update streaming functionality by @skiniks
- simplify fallback handling in rendering components by @skiniks
- streamline module specifier generation and enhance promise resolution handling by @skiniks
- improve promise handling in StreamingRenderer by @skiniks
- update dependency registration logic in RscRenderer by @skiniks
- optimize dependency registration in RscRenderer by @skiniks

### ⚙️ Miscellaneous Tasks

- update dependencies and improve streaming support by @skiniks
- update rari package version and dependencies by @skiniks
- update oxlint and rolldown-vite dependencies across multiple packages by @skiniks
- update rolldown-vite version to 7.1.2 and enhance streaming functionality by @skiniks
- update changelog generation settings and remove unused changelog files by @skiniks
- update esbuild and eslint-react dependencies by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.1.5...v0.2.0
## [0.1.5] - 2025-08-07

### ⚙️ Miscellaneous Tasks

- update rari dependency versions to 0.1.4 in examples and docs, and bump optional dependencies to 0.1.5 in rari package by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/rari@0.1.4...v0.1.5
## [rari@0.1.4] - 2025-08-07

### ⚙️ Miscellaneous Tasks

- add git-cliff configuration and update changelog generation by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.1.3...rari@0.1.4
