## [0.7.5] - 2026-01-19

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.7.5
## [0.7.5] - 2026-01-19

### 🚀 Features

- *(rari)* Refactor routing and rendering architecture with improved component resolution
- *(rari,web)* Add custom define option and integrate sponsor link into navigation
- *(og)* Add ImageResponse documentation

### 📚 Documentation

- Update README with revised performance metrics and streamlined content

### ⚙️ Miscellaneous Tasks

- *(tooling)* Migrate release and binary preparation scripts to Rust
- *(tooling)* Migrate git-cliff from npm to system binary
- *(changelog)* Exclude non-core directories from git-cliff
- *(lint)* Consolidate eslint configuration and update tooling
- *(build)* Consolidate workspace dependencies and enhance clippy linting
- *(justfile)* Enhance testing infrastructure with nextest
- *(release)* Bump rari binaries version to 0.7.5
## [rari@0.7.4] - 2026-01-15

### 💼 Other

- Rari@0.7.4

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump binary version to 0.7.4
## [0.7.4] - 2026-01-15

### 🚀 Features

- *(rari)* Optimize component resolution and rendering performance

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump binary version to 0.7.4
## [create-rari-app@0.3.4] - 2026-01-14

### 💼 Other

- Create-rari-app@0.3.4
## [rari@0.7.3] - 2026-01-14

### 💼 Other

- Rari@0.7.3

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump binary version to 0.7.3
## [0.7.3] - 2026-01-14

### 🚀 Features

- *(rari)* Enhance open graph and twitter metadata handling
- *(rari)* Add body scripts extraction and injection support

### 🚜 Refactor

- *(docs)* Restructure documentation and add blog section
- *(web)* Rename docs directory to web
- Rename docs directory to web and update references

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump binary version to 0.7.3
## [create-rari-app@0.3.3] - 2026-01-13

### 💼 Other

- Create-rari-app@0.3.3
## [rari@0.7.2] - 2026-01-13

### 💼 Other

- Rari@0.7.2
## [0.7.2] - 2026-01-13

### 💼 Other

- Rari@0.7.2

### ⚙️ Miscellaneous Tasks

- *(docs)* Update rari dependency to 0.7.1
- *(release)* Downgrade ubuntu runner versions for compatibility
- *(release)* Update ubuntu runner to arm64 compatible version
## [create-rari-app@0.3.2] - 2026-01-13

### 💼 Other

- Create-rari-app@0.3.2
## [rari@0.7.1] - 2026-01-13

### 💼 Other

- Rari@0.7.1

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.7.1
## [0.7.1] - 2026-01-13

### 💼 Other

- Binaries 0.7.1

### 📚 Documentation

- Update rari dependency to published version and fix typo

### ⚙️ Miscellaneous Tasks

- Remove dav1d dependency and update dependencies
## [create-rari-app@0.3.1] - 2026-01-13

### 💼 Other

- Create-rari-app@0.3.1
## [rari@0.7.0] - 2026-01-13

### 💼 Other

- Rari@0.7.0

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.7.0
## [0.7.0] - 2026-01-13

### 🚀 Features

- *(image)* Add image optimization and caching system
- *(og)* Add open graph image generation with dynamic rendering
- *(security)* Add IP-based rate limiting for critical endpoints
- *(image)* Add local image pattern matching and AVIF native support
- *(og)* Inject generated OG images into page metadata
- *(security)* Add environment-aware rate limiting configuration
- *(image)* Change default image format from WebP to AVIF
- *(security)* Make CSRF protection optional with environment configuration

### 🐛 Bug Fixes

- *(rsc)* Handle stale content and missing promises gracefully
- *(docs)* Fix nested backticks in docs mdx

### 🚜 Refactor

- *(error)* Simplify error enum and clippy configuration
- *(hmr)* Consolidate HMR handlers into unified action endpoint
- *(server)* Move register endpoints to development-only routes
- *(rsc)* Stream RSC responses directly without buffering
- *(sync)* Replace std::sync with parking_lot for better performance
- *(vite)* Fix client component HMR handling
- *(image)* Rename priority to preload and add custom loader support
- *(api)* Consolidate internal routes under /_rari namespace
- *(path)* Remove unused custom paths constructor and test
- *(layout)* Remove unused skeleton validation and layout structure methods
- *(spam_blocker)* Add block reason tracking and improve response handling
- *(server)* Improve app router error handling and validation
- *(cache)* Remove production environment check from caching logic
- *(rendering)* Improve HTML structure handling and asset extraction
- *(config)* Migrate CSP and rate limit config from environment variables to manifest
- *(config)* Remove TOML file support and simplify configuration loading

### ⚡ Performance

- *(rsc)* Replace string allocations with cow-utils for efficiency

### 🎨 Styling

- *(rsc)* Remove unnecessary braces from single-statement conditionals
- *(node)* Fix formatting in promisify callback handler

### ⚙️ Miscellaneous Tasks

- *(router)* Remove item from skip directories list
- *(release)* Bump binary version to 0.7.0
- *(release)* Add libdav1d-dev dependency for Linux builds
- *(release)* Add arm64 libdav1d-dev for cross-compilation
- *(release)* Use cross tool for aarch64 builds with native dependencies
- *(release)* Upgrade to Ubuntu 24.04 for dav1d 1.3.0+ support
- *(release)* Use native ARM64 runners instead of cross-compilation
- *(release)* Add dav1d installation for macOS and Windows
- *(release)* Add fontconfig dependency for text rendering
- *(release)* Suppress macOS brew warnings for already installed packages
- *(release)* Fix ARM64 runner label to ubuntu-24.04-arm
## [rari@0.6.1] - 2026-01-08

### 💼 Other

- Rari@0.6.1
## [0.6.1] - 2026-01-08

### 🚀 Features

- *(server)* Implement separate rate limiting for revalidation endpoints
- *(server)* Expand spam blocker patterns for WordPress and IDE configs

### 💼 Other

- V0.6.1

### 🚜 Refactor

- *(docs)* Extract clipboard logic into reusable hook
- *(docs)* Remove unused syntax highlighter language imports

### 🎨 Styling

- *(docs)* Improve responsive layout and text overflow handling
- *(docs)* Remove unnecessary blank lines for consistency
## [create-rari-app@0.3.0] - 2026-01-08

### 💼 Other

- Create-rari-app@0.3.0
## [rari@0.6.0] - 2026-01-08

### 💼 Other

- Rari@0.6.0
## [0.6.0] - 2026-01-08

### 🚀 Features

- *(docs)* Add package manager tabs and terminal blocks to MDX
- *(docs)* Add footer component with social links and improve layout structure
- *(docs)* Enhance homepage metadata and add GitHub stars to footer
- *(docs)* Add custom CodeBlock component with syntax highlighting
- *(docs)* Extract inline SVG icons into reusable components
- *(docs)* Add Close icon and improve mobile sidebar navigation
- *(docs)* Add Deno support to getting-started guide
- *(docs)* Add Heart icon and enhance homepage quick start section
- *(docs)* Add React, TypeScript, and Vite file icons to CodeBlock
- *(proxy)* Add request/response proxy middleware and runtime execution
- *(middleware)* Add spam blocker middleware for request filtering
- *(robots)* Add robots.txt generation support
- *(metadata)* Add comprehensive metadata support for icons, theme, and apple web app
- *(vite)* Skip robots and sitemap files in server component scanning

### 🚜 Refactor

- *(router)* Remove loading component map generation

### 🎨 Styling

- *(docs)* Simplify error handling in MdxRenderer
- *(docs)* Refactor footer copyright text and remove npm link
- *(docs)* Remove unnecessary braces from conditional statements
- *(router)* Remove unnecessary braces from conditional statements
- *(rari)* Remove unnecessary braces from conditional statements
- *(router)* Remove unnecessary braces from conditional statements
- Remove unnecessary braces from conditional statements
- Remove unnecessary braces from conditional statements
- *(vite)* Remove unnecessary braces from conditional statements
- *(rari)* Remove unnecessary braces from conditional statements
- *(rari)* Remove unnecessary braces from conditional statements
- *(router)* Remove unnecessary braces from conditional statements
- *(vite)* Remove esbuildOptions deprecation warning suppression
- *(rari)* Remove unnecessary blank lines from conditional statements
- *(rari)* Remove unnecessary blank lines from vite plugin
- *(docs)* Remove unnecessary blank lines from highlight-command
- *(docs)* Remove unnecessary blank lines from CodeBlock
- *(docs)* Update sponsor button heart icon color

### ⚙️ Miscellaneous Tasks

- *(knip)* Remove unused crates ignore entry from config
- *(rari)* Remove useActionState hook and exports
- *(rari)* Remove file extensions from mdx exports
- *(rari)* Remove AppRouterProvider export from package.json
- *(rari)* Remove fsevents from external dependencies
- *(tsconfig)* Consolidate TypeScript configuration files
- *(create-rari-app)* Consolidate TypeScript configuration files
- *(vite)* Remove external dependencies configuration
- *(build)* Reorganize server manifests into dist/server directory
- *(eslint)* Remove markdown-specific ESLint rule overrides
- *(release)* Binaries bump version to 0.6.0
## [create-rari-app@0.2.15] - 2026-01-03

### 💼 Other

- Create-rari-app@0.2.15

### ⚙️ Miscellaneous Tasks

- *(knip)* Remove unused router and vite entries from ignore list
- *(knip)* Remove unused ignore entries from config
- *(create-rari-app)* Remove unused linting and react plugin dependencies
- *(create-rari-app)* Simplify railway.toml configuration
## [rari@0.5.30] - 2026-01-03

### 🚀 Features

- *(rsc)* Add client-side RSC fetching and dev server proxying

### 💼 Other

- Rari@0.5.30

### 🚜 Refactor

- *(router)* Remove unused exports and internal utilities
- *(router)* Remove legacy layout and runtime management systems

### ⚙️ Miscellaneous Tasks

- *(router)* Remove router index barrel export and consolidate exports
## [create-rari-app@0.2.14] - 2026-01-02

### 💼 Other

- Create-rari-app@0.2.14
## [rari@0.5.29] - 2026-01-02

### 💼 Other

- Rari@0.5.29
## [0.5.23] - 2026-01-02

### 🚀 Features

- *(rsc)* Improve streaming updates
- *(rsc)* Enhance lazy loading and streaming completion handling
- *(rsc-client-runtime)* Improve RSC row parsing
- *(rsc)* Optimize lazy loading and promise resolution handling
- *(rari)* Implement partial hydration and dynamic module loading
- *(rsc)* Implement lazy promise resolution and streaming suspense
- *(rsc)* Rename internal module markers from double underscore to tilde prefix
- *(rari)* Add chunked transfer encoding to streaming RSC responses
- *(server)* Add stream compression support with zstd, brotli, and gzip
- *(docs)* Make DocPage component async

### 🐛 Bug Fixes

- *(rsc)* Correct row ID generation in serializer
- *(rari)* Remove debug console.warn statements from RSC client

### 🚜 Refactor

- *(rsc)* Extract HTML closing generation into dedicated method
- *(server)* Remove x-accel-buffering header from streaming responses
- *(rsc)* Remove suspense module and consolidate functionality
- *(docs)* Simplify doc page layout and remove wrapper divs

### 🎨 Styling

- *(rari)* Remove unnecessary comment

### ⚙️ Miscellaneous Tasks

- *(docs)* Migrate deployment config from railway to railpack
- *(docs)* Config fix
- *(docs)* Remove railpack builder config, it's default
- *(docs)* Reverting config change
- *(workspace)* Remove catalogMode
- *(release)* Bump rari binary version to 0.5.23
## [rari@0.5.28] - 2025-12-24

### 💼 Other

- Rari@0.5.28
## [0.5.22] - 2025-12-24

### 🚀 Features

- *(server)* Invalidate caches on server action redirects

### 🐛 Bug Fixes

- *(server)* Return correct HTTP status codes for not-found routes

### 🚜 Refactor

- *(module-loader)* Improve dynamic import path resolution logic

### ⚙️ Miscellaneous Tasks

- *(release)* Bump rari version to 0.5.22
## [rari@0.5.27] - 2025-12-24

### 💼 Other

- Rari@0.5.27
## [0.5.21] - 2025-12-24

### 🚀 Features

- *(server)* Implement on-demand revalidation and response caching

### ⚙️ Miscellaneous Tasks

- *(release)* Bump rari version to 0.5.21
## [rari@0.5.26] - 2025-12-23

### 💼 Other

- Rari@0.5.26
## [0.5.20] - 2025-12-23

### 🚀 Features

- *(routing)* Implement dynamic route info endpoint and remove manifest injection

### 🐛 Bug Fixes

- *(server)* Block access to app-routes.json in request handlers

### 🎨 Styling

- *(docs)* Simplify error handling in metadata generation

### ⚙️ Miscellaneous Tasks

- *(release)* Bump rari version to 0.5.20
## [create-rari-app@0.2.13] - 2025-12-23

### 💼 Other

- Create-rari-app@0.2.13
## [rari@0.5.25] - 2025-12-23

### 💼 Other

- Rari@0.5.25

### 🎨 Styling

- Simplify JSX formatting and expand markdown linting scope
## [0.5.19] - 2025-12-23

### 🚀 Features

- *(rsc)* Add RSC wire format payload and manifest embedding
- *(mdx)* Replace mdx-remote with native @mdx-js/mdx compilation

### 🐛 Bug Fixes

- *(rsc)* Correct React element symbol and add transitional element support
- *(rsc)* Correct React symbol property names from single to double dollar signs
- *(rsc)* Update React element symbol to use transitional variant
- *(rsc)* Remove ref property from React element objects
- *(rsc)* Remove ref property from React element objects
- *(rsc)* Update React element symbol to use transitional variant
- *(rsc)* Remove ref property and forwardRef from React elements

### 🚜 Refactor

- *(docs)* Extract frontmatter parsing into dedicated utility

### 📚 Documentation

- *(getting-started)* Simplify JSX formatting in code examples
- *(homepage)* Remove unused PageProps import and parameter

### 🎨 Styling

- *(rsc)* Remove unnecessary braces from single-line conditional

### ⚙️ Miscellaneous Tasks

- *(eslint)* Remove pnpm catalog enforcement rule override
- *(eslint)* Move react-refresh rule disable to config
- *(release)* Bump rari version to 0.5.19
## [create-rari-app@0.2.12] - 2025-12-20

### 💼 Other

- Create-rari-app@0.2.12
## [rari@0.5.24] - 2025-12-20

### 💼 Other

- Rari@0.5.24
## [0.5.18] - 2025-12-20

### 🚜 Refactor

- *(rsc)* Rename client component registry globals to use tilde prefix
- *(rsc)* Migrate global namespace from __rari to ~rari
- *(rsc)* Migrate global namespace from __rari to ~rari
- *(rsc)* Migrate global namespace from __rsc to ~rsc
- *(rsc)* Remove Counter component special handling from RSC traversal
- *(rsc)* Migrate global namespace from double underscore to tilde prefix
- *(rsc)* Migrate global namespace from double underscore to tilde prefix
- *(rsc)* Migrate global namespace from double underscore to tilde prefix
- *(runtime)* Migrate error handling globals from double underscore to tilde prefix
- *(rsc)* Migrate registry proxy global from double underscore to tilde prefix
- *(hmr)* Improve component specifier handling and remove debug logging
- *(rsc)* Remove debug logging statements across codebase
- *(runtime)* Remove lifecycle logging from LayoutWrapper

### ⚙️ Miscellaneous Tasks

- *(logging)* Remove info-level logging statements across codebase
- *(logging)* Remove warn-level logging statements
- *(logging)* Remove warn-level logging and upgrade to error-level where appropriate
- *(logging)* Remove warn-level logging and unused variables
- *(release)* Bump version to 0.5.18
## [rari@0.5.23] - 2025-12-18

### 💼 Other

- Rari@0.5.23

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.5.17
## [0.5.17] - 2025-12-18

### 🚀 Features

- *(rsc)* Implement getData-based page not found detection

### 💼 Other

- V0.5.17
## [rari@0.5.22] - 2025-12-17

### 💼 Other

- Rari@0.5.22

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.5.16
## [0.5.16] - 2025-12-17

### 🐛 Bug Fixes

- *(server)* Return 404 status code for not found routes

### 💼 Other

- V0.5.16
## [rari@0.5.21] - 2025-12-17

### 💼 Other

- Rari@0.5.21

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.5.15
## [0.5.15] - 2025-12-17

### 🐛 Bug Fixes

- *(server)* Block access to sensitive internal files
- *(server)* Block access to sensitive internal files

### 💼 Other

- Rari@0.5.15

### 🚜 Refactor

- *(docs)* Migrate markdown parser from markdown-it to marked
- *(server)* Separate stylesheet and script injection into head and body
- *(rsc)* Standardize boundary ID prop naming to ~boundaryId
- *(rsc)* Standardize pre-serialized suspense prop naming
- *(rsc)* Standardize client component marker prop naming
- *(rsc)* Standardize data attribute naming with tilde prefix

### ⚙️ Miscellaneous Tasks

- *(docs)* Add marked to external dependencies in vite config
## [rari@0.5.20] - 2025-12-17

### 💼 Other

- Rari@0.5.20

### ⚙️ Miscellaneous Tasks

- *(release)* Bump rari to 0.5.14
## [0.5.14] - 2025-12-17

### 🚀 Features

- *(rari)* Add external module configuration for server builds
- *(server)* Improve server action module loading with ESM support

### 🚜 Refactor

- *(rsc)* Remove legacy module fallback patterns and simplify runtime
- *(vite)* Simplify node imports transformation in server build

### 📚 Documentation

- *(getting-started)* Standardize pnpm usage and add path aliases
- *(getting-started)* Improve example with real-world API usage and styling

### 🎨 Styling

- *(docs)* Improve prose list styling and readability

### ⚙️ Miscellaneous Tasks

- *(release)* Bump rari to 0.5.14
## [create-rari-app@0.2.11] - 2025-12-17

### 🚀 Features

- *(runtime)* Add module loading and component initialization improvements

### 💼 Other

- Create-rari-app@0.2.11

### 🚜 Refactor

- *(vite)* Remove unnecessary banners and optimize minification settings

### ⚙️ Miscellaneous Tasks

- *(create-rari-app)* Add predev script to default template
## [rari@0.5.19] - 2025-12-16

### 💼 Other

- Rari@0.5.19
## [0.5.13] - 2025-12-16

### 🚀 Features

- *(docs)* Add dynamic metadata generation and route path conversion

### 📚 Documentation

- *(getting-started)* Update prerequisites and simplify project structure
## [rari@0.5.18] - 2025-12-16

### 💼 Other

- Rari@0.5.18
## [0.5.12] - 2025-12-16

### 🐛 Bug Fixes

- *(server)* Correct asset injection detection logic
## [rari@0.5.17] - 2025-12-16

### 💼 Other

- Rari@0.5.17

### 📚 Documentation

- *(getting-started)* Simplify code examples

### 🎨 Styling

- *(eslint)* Disable dangerously-set-innerhtml rule for docs
## [0.5.11] - 2025-12-16

### 🚀 Features

- *(metadata)* Add page metadata collection and injection system

### 💼 Other

- Create-rari-app@0.2.10

### 🚜 Refactor

- *(docs)* Extract shiki highlighter to dedicated module
- *(server)* Extract html wrapping logic into function

### ⚙️ Miscellaneous Tasks

- *(create-rari-app)* Update bin entry point to ESM format
## [rari@0.5.16] - 2025-12-13

### 💼 Other

- Rari@0.5.16

### 🚜 Refactor

- *(docs)* Migrate from shiki to @shikijs modular packages
- *(docs,examples)* Migrate to path aliases
## [create-rari-app@0.2.9] - 2025-12-12

### 💼 Other

- Create-rari-app@0.2.9
## [rari@0.5.15] - 2025-12-12

### 💼 Other

- Rari@0.5.15
## [0.5.10] - 2025-12-12

### 🚀 Features

- *(rsc)* Enhance prop serialization and client component registration
- *(runtime)* Enhance Node.js compatibility stubs for fs, path, and process

### 🐛 Bug Fixes

- *(router)* Improve component loading fallback logic

### 🚜 Refactor

- *(docs,rari)* Improve markdown rendering and module resolution

### ⚙️ Miscellaneous Tasks

- *(release)* Bump rari to 0.5.10
## [rari@0.5.14] - 2025-12-11

### 💼 Other

- Rari@0.5.14
## [create-rari-app@0.2.8] - 2025-12-11

### 💼 Other

- Create-rari-app@0.2.8
## [0.5.9] - 2025-12-11

### 🐛 Bug Fixes

- *(web)* Add URL and URLSearchParams to global scope

### ⚙️ Miscellaneous Tasks

- Remove Windows build test workflow
- *(release)* Bump rari to 0.5.9
## [create-rari-app@0.2.7] - 2025-12-11

### 💼 Other

- Create-rari-app@0.2.7
## [rari@0.5.13] - 2025-12-11

### 💼 Other

- Rari@0.5.13

### ⚙️ Miscellaneous Tasks

- *(release)* Bump rari platform dependencies to 0.5.8
## [0.5.8] - 2025-12-11

### 🐛 Bug Fixes

- Add Win32_System_SystemInformation feature for windows-sys
- *(deps)* Expand windows-sys features for Windows API support
- *(deps)* Add Win32_System_SystemInformation feature to windows-sys
- *(deps)* Add Win32_Networking_WinSock feature to windows-sys

### ⚙️ Miscellaneous Tasks

- Add Windows build test workflow
- *(release)* Bump version to 0.5.8
## [0.5.7] - 2025-12-11

### 🐛 Bug Fixes

- *(cli)* Guard main execution to prevent running when imported as module

### 🚜 Refactor

- *(runtime)* Remove unnecessary clone on cache options

### ⚙️ Miscellaneous Tasks

- *(server)* Simplify startup logging and remove verbose debug output
- *(scripts)* Add postinstall script to build on dependency installation
- *(rari)* Disable doctests and remove bin tests
- *(server)* Remove verbose startup completion messages
- *(release)* Bump version to 0.5.7
## [rari@0.5.12] - 2025-12-11

### 🚀 Features

- *(router)* Improve loading component handling and validation

### 💼 Other

- Rari@0.5.12
## [rari@0.5.11] - 2025-12-11

### 🚀 Features

- *(rari)* Improve client-server routing and add manifest middleware
- *(router)* Convert loading component modules to Map

### 💼 Other

- Rari@0.5.11

### ⚙️ Miscellaneous Tasks

- Upgrade dependencies and remove unused packages
## [create-rari-app@0.2.6] - 2025-12-09

### 💼 Other

- Create-rari-app@0.2.6
## [rari@0.5.10] - 2025-12-09

### 💼 Other

- Rari@0.5.10

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.5.6
## [0.5.6] - 2025-12-09

### 🚀 Features

- *(server)* Add path validation utilities for security
- *(server)* Add path validation to HMR component reload handler
- *(server)* Add CSRF token protection to server actions
- *(server)* Add CSRF token generation and injection to HTML responses
- *(server)* Integrate CSRF token generation into RSC rendering pipeline
- *(server)* Add configurable CORS support with origin validation
- *(server)* Add input validation with configurable depth and size limits
- *(server)* Add redirect URL validation with configurable host allowlist
- *(server)* Add configurable Content Security Policy support
- *(server)* Enhance security headers and simplify CSP configuration
- *(server)* Add granular body size limits for API routes
- *(error)* Add HTTP status codes and safe error messages for API responses
- *(server)* Add configurable rate limiting with per-IP tracking
- *(csrf)* Move CSRF token generation to client-side runtime
- *(actions)* Add reserved export name validation for server actions
- *(rsc)* Add special value and reference type parsing for wire format

### 🎨 Styling

- *(docs)* Simplify Tailwind class syntax in Sidebar component

### ⚙️ Miscellaneous Tasks

- Upgrade dependencies and enhance linting rules
- Bump rari version to 0.5.6
## [rari@0.5.9] - 2025-12-09

### 💼 Other

- Rari@0.5.9

### ⚙️ Miscellaneous Tasks

- *(release)* Bump binaries to 0.5.5
## [0.5.5] - 2025-12-09

### 🚀 Features

- *(rsc)* Extract RSC wire format escaping utilities

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to 0.5.5
- *(release)* Update macOS runner to macos-15-intel
## [rari@0.5.8] - 2025-12-04

### 💼 Other

- Rari@0.5.8

### ⚙️ Miscellaneous Tasks

- *(release)* Bump binaries to 0.5.4
## [0.5.4] - 2025-12-04

### 🚀 Features

- *(rsc)* Extract client-side RSC runtime from inline and update rendering pipeline
- *(runtime)* Extract client-side entry point and RSC wire format parser

### 🚜 Refactor

- *(rsc)* Reorganize module structure for rendering pipeline
- *(runtime)* Use StreamOpState from ops module
- *(server)* Reorganize vite proxy into module structure
- *(server)* Reorganize types
- *(server)* Reorganize module structure
- *(runtime)* Move transpile module into utils
- *(rsc)* Extract JavaScript initialization and rendering scripts

### 🎨 Styling

- *(runtime)* Clean up linting directives and modernize string formatting

### ⚙️ Miscellaneous Tasks

- *(rari)* Configure cargo-machete to ignore num-bigint-dig dependency
- *(release)* Bump version to 0.5.4
## [rari@0.5.7] - 2025-12-04

### 💼 Other

- Rari@0.5.7
## [0.5.3] - 2025-12-03

### 📚 Documentation

- *(railway)* Restructure configuration for improved readability
- *(railway)* Add pnpm install command to nixpacks build phase
- *(railway)* Extract nixpacks configuration to separate file

### ⚙️ Miscellaneous Tasks

- *(docs)* Update package manager and Node.js version requirements
- *(release)* Bump version to 0.5.3
## [create-rari-app@0.2.5] - 2025-12-03

### 💼 Other

- Create-rari-app@0.2.5
## [rari@0.5.6] - 2025-12-03

### 💼 Other

- Rari@0.5.6
## [0.5.2] - 2025-12-03

### 🐛 Bug Fixes

- *(router)* Add Vite ignore comments to dynamic imports
- *(security)* Prevent prototype pollution in server actions (CVE-2025-55182)

### 🚜 Refactor

- *(rsc)* Extract and reorganize type definitions and fix dupes
- *(server)* Reorganize server module structure
- *(runtime)* Extract runtime_factory into submodules
- *(module_loader)* Extract module loader into submodules
- *(runtime)* Extract runtime submodules into new org
- *(rsc)* Reorganize RSC module structure into submodules
- *(rsc)* Extract streaming module scripts into separate files
- *(rsc)* Extract streaming submodules into separate files
- *(rsc)* Extract renderer scripts and constants into separate files
- *(rsc)* Extract renderer core logic and utilities into submodules
- *(rsc)* Extract layout renderer scripts into separate files
- *(layout_renderer)* Extract core logic and utilities into submodules
- *(rsc)* Remove unused fields and dead code
- *(runtime)* Remove unused fields and dead code
- *(rsc,runtime)* Remove auto-register logic and hardcoded paths

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to 0.5.2
## [rari@0.5.5] - 2025-12-02

### 💼 Other

- Rari@0.5.5
## [0.5.1] - 2025-12-02

### 🚜 Refactor

- *(rsc)* Replace timestamp-based key generation with counter for stable key gen

### ⚙️ Miscellaneous Tasks

- Bump binary packages to v0.5.1
## [rari@0.5.4] - 2025-11-24

### 💼 Other

- Rari@0.5.4
- Rari@0.5.4

### 🚜 Refactor

- *(cli)* Fix for issue #45
## [rari@0.5.3] - 2025-11-24

### 💼 Other

- Rari@0.5.3
- Rari@0.5.3
- Rari@0.5.3

### ⚙️ Miscellaneous Tasks

- *(build)* Update export file extensions from .mjs to .js
- *(build)* Adding fixedExtension to tsdown config
## [rari@0.5.2] - 2025-11-24

### 💼 Other

- Rari@0.5.2

### ⚙️ Miscellaneous Tasks

- *(build)* Update export file extensions
## [rari@0.5.1] - 2025-11-24

### 💼 Other

- Rari@0.5.1

### ⚙️ Miscellaneous Tasks

- *(build)* Update export file extensions
## [rari@0.5.0] - 2025-11-24

### 💼 Other

- Rari@0.5.0
## [create-rari-app@0.2.4] - 2025-11-24

### 💼 Other

- Create-rari-app@0.2.4
## [0.5.0] - 2025-11-24

### 🚀 Features

- *(error)* Add comprehensive streaming and loading state error handling
- *(rsc)* Add RSC wire format parser and streaming layout renderer
- *(rsc)* Refactor streaming render completion to use async/await with channel signaling
- *(router)* Implement client-side routing with layout management and error handling

### 🐛 Bug Fixes

- *(docs)* Update LayoutProps import source from server to client

### 🚜 Refactor

- *(vite)* Simplify client-side rendering logic
- *(exports)* Rename server export to vite for clarity

### 📚 Documentation

- *(readme)* Update features section with completed streaming capabilities

### 🧪 Testing

- *(layout_renderer)* Remove obsolete assertion messages from tests

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to 0.5.0
## [create-rari-app@0.2.3] - 2025-11-11

### 💼 Other

- Create-rari-app@0.2.2
- Create-rari-app@0.2.3

### ⚙️ Miscellaneous Tasks

- Show compiler output in `prepare-binaries.js`
## [rari@0.4.0] - 2025-10-30

### 💼 Other

- Rari@0.4.0
## [0.4.0] - 2025-10-30

### 🚀 Features

- *(runtime)* Add API request bridge for Rari server runtime
- *(runtime)* Upgrade Deno dependencies and improve module loading

### 🎨 Styling

- *(template)* Add eslint-disable comment for object-curly-spacing rule

### ⚙️ Miscellaneous Tasks

- *(docs)* Optimize Google Analytics and performance loading
- Bump version to 0.4.0
## [rari@0.3.3] - 2025-10-28

### 💼 Other

- Rari@0.3.3
## [0.3.3] - 2025-10-28

### 💼 Other

- Rari@0.3.3

### 🚜 Refactor

- *(rsc)* Remove JSX transformation from core  and simplify dependency extraction
## [rari@0.3.2] - 2025-10-28

### 💼 Other

- Rari@0.3.2
## [0.3.2] - 2025-10-28

### 🐛 Bug Fixes

- Prevent false ES module detection when code contains "export" in strings

### 💼 Other

- Rari@0.3.2
## [rari@0.3.1] - 2025-10-24

### 💼 Other

- Rari@0.3.1
## [0.3.1] - 2025-10-24

### 🚀 Features

- *(rari)* Add pathname support for layouts and server components

### 💼 Other

- Rari@0.3.1
- Rari@0.3.1

### 🚜 Refactor

- *(docs)* Simplify mobile navigation using CSS checkbox toggle

### 📚 Documentation

- *(readme)* Update docs
## [create-rari-app@0.2.1] - 2025-10-21

### 🚀 Features

- *(create-rari-app)* Enhance project creation and build process

### 💼 Other

- Create-rari-app@0.2.1

### ⚙️ Miscellaneous Tasks

- *(docs)* Update rari package dependency to latest
- *(railway)* Update build configuration for Railway deployment
- *(railway)* Update Node.js version for deployment
## [create-rari-app@0.2.0] - 2025-10-21

### 💼 Other

- Create-rari-app@0.2.0
## [rari@0.3.0] - 2025-10-21

### 💼 Other

- Rari@0.3.0
## [0.3.0] - 2025-10-21

### 🚀 Features

- Implement app router with dynamic routing, layouts, and error handling; add example application
- Add static asset serving and production handling in server
- Implement server actions and enhance form handling with progressive enhancement in example app
- *(HMR)* Implement component reload functionality and error handling
- *(ssr)* Introduce server-side rendering capabilities and configuration options
- *(ssr)* Enhance SSR rendering with raw content handling and improved child rendering logic
- *(ssr)* Enhance script and link tag extraction with improved handling
- *(rsc)* Enhance layout rendering with performance metrics and direct HTML rendering
- *(rsc)* Introduce direct HTML rendering and enhance layout rendering with caching
- *(runtime)* Enhance request context handling and runtime integration
- *(rsc)* Introduce RSC HTML rendering and refactor SSR components
- *(example)* Upgrade app-router example with Tailwind CSS and styling improvements
- *(docs)* Migrate documentation to App Router structure
- *(template)* Migrate default template to App Router structure
- *(router)* Add support for global and scoped not-found routes
- *(build)* Add React OXC plugin and update Vite config

### 🐛 Bug Fixes

- Correct component path replacement
- *(docs)* Improve error handling and styling in markdown rendering

### 🚜 Refactor

- Remove 'use server' directives from components and update documentation to clarify server/client component usage
- Replace entry-client.tsx with virtual import for client components
- Simplify server component detection logic by removing redundant checks
- Streamline rari router plugin by removing unused options and simplifying route generation logic
- Clean up routing code by deleting obsolete files
- Remove obsolete '.rari' directory references and streamline routing logic
- Unify cache configuration types and simplify route matching logic in server module
- Enhance middleware handling and dynamic Vite port config
- Enhance async handling in layout and streaming renderers, update RSC traversal logic for better error handling
- Remove unused test handler and direct registration route
- Expanding HMR for app router support
- *(error)* Improve ModuleReloadError handling with boxed error variant
- *(AppRouterHMRProvider)* Streamline event handling and enhance RSC payload management
- *(ssr)* Reorganize SSR rendering logic and improve error handling
- *(rsc)* Remove renderer pool and simplify layout rendering
- *(rari)* Simplify server component checks and remove unused directory validations
- *(rsc)* Simplify caching logic and remove route-specific cache exclusions
- *(rsc)* Rename RSC function registration global to improve clarity
- *(rsc)* Remove HTML diagnostics and simplify rendering logging
- *(rsc)* Enhance HTML template handling and logging
- *(rsc)* Consolidate HTML rendering logic and improve cache management
- *(rsc)* Simplify component registration and remove redundant hash-based keys
- *(rsc)* Rename RSC rendering functions to follow consistent naming convention
- *(rsc)* Improve component invalidation and registration mechanisms
- *(rsc)* Improve component import transformation for global component resolution
- *(rsc)* Enhance HTML rendering with raw content support and vendor prefix handling
- *(example)* Revamp app-router example with enhanced layout, styling
- *(rsc)* Improve client-side rendering and hydration logic
- *(rsc)* Fix hydration logic in renderApp
- *(server)* Enhance static file handling and security configuration
- *(server)* Enhance Content Security Policy configuration
- *(rsc)* Enhance layout renderer test cases with not_found support

### 📚 Documentation

- *(readme)* Update performance metrics and project overview
- *(readme)* Update project structure and example code
- *(rari)* Add comprehensive package metadata for Cargo.toml

### 🎨 Styling

- *(tailwind)* Replace deprecated flex-shrink-0 with shrink-0 utility class
- *(not-found)* Update 404 page link styling and hover states

### ⚙️ Miscellaneous Tasks

- Update rari dependency to version 0.2.24
- Update build command and node version in railway.toml
- Update dependencies in pnpm-lock.yaml and package.json, and modify build command in railway.toml
- Update dependencies in examples and pnpm-lock.yaml, replace vite with rolldown-vite in app-router-example
- Update server and Vite configuration ports to 5173
- *(linting)* Update ESLint and Oxlint configuration
- *(build)* Update build output directory from .rari to dist
- *(dependencies)* Update project dependencies to latest versions
- *(dependencies)* Update project dependencies
- Fix linting errors
- *(docs)* Remove unused Vite environment type declarations
- *(dependencies)* Update project dependencies and linting config
- *(rari)* Bump package version to 0.3.0
- *(rari)* Bump package version to 0.3.0
## [create-rari-app@0.1.12] - 2025-09-23

### 💼 Other

- Create-rari-app@0.1.12
## [rari@0.2.24] - 2025-09-23

### 💼 Other

- Rari@0.2.24

### ⚙️ Miscellaneous Tasks

- Update rari dependency to version 0.2.23
- Update dependencies across multiple packages to latest versions
## [rari@0.2.23] - 2025-09-23

### 💼 Other

- Rari@0.2.23
## [0.2.16] - 2025-09-23

### 🚀 Features

- Add support for component initial loading state and enhance HMR functionality

### 🚜 Refactor

- Enhance error handling and retry logic in component rendering and registration processes

### ⚙️ Miscellaneous Tasks

- Update rari dependency to version 0.2.22
- Bump version to 0.2.16 for multiple Rari packages
## [rari@0.2.22] - 2025-09-04

### 💼 Other

- Rari@0.2.22

### 🚜 Refactor

- Improve formatting and error handling in extractCacheConfigFromContent function

### ⚙️ Miscellaneous Tasks

- Update rari dependency to version 0.2.21
## [rari@0.2.21] - 2025-09-04

### 💼 Other

- Rari@0.2.21

### ⚙️ Miscellaneous Tasks

- Update rari dependencies to version 0.2.15 and remove unused client-dev entry
## [0.2.15] - 2025-09-04

### ⚙️ Miscellaneous Tasks

- Update rari dependency to version 0.2.20
- Update rari to version 0.2.8 and bump dependencies across multiple packages
## [rari@0.2.20] - 2025-09-04

### 💼 Other

- Rari@0.2.20
## [create-rari-app@0.1.11] - 2025-09-04

### 💼 Other

- Create-rari-app@0.1.11

### 🚜 Refactor

- Remove unused router components and functions, simplify package.json dependencies

### ⚙️ Miscellaneous Tasks

- Update rari to version 0.2.19
- Update ESLint configuration, remove unused dependencies, and refactor icon exports to default
- Update dependencies across multiple packages to latest versions
## [rari@0.2.19] - 2025-09-02

### 💼 Other

- Rari@0.2.19

### ⚙️ Miscellaneous Tasks

- Update optionalDependencies for rari to version 0.2.14
## [0.2.14] - 2025-09-02

### ⚙️ Miscellaneous Tasks

- Bump rari version to 0.2.18
- Update rari to version 0.2.7, add caching configuration support, and remove macOS Rust version override in release workflow
- Bump rari version to 0.2.14 across all platforms
## [create-rari-app@0.1.10] - 2025-08-22

### 💼 Other

- Create-rari-app@0.1.10
## [rari@0.2.18] - 2025-08-22

### 💼 Other

- Rari@0.2.18
## [0.2.13] - 2025-08-22

### ⚙️ Miscellaneous Tasks

- Bump rari version to 0.2.6 and update type annotations
- Update dependencies
- Bump rari version to 0.2.13 across all platforms
## [rari@0.2.17] - 2025-08-21

### 💼 Other

- Rari@0.2.17
## [0.2.12] - 2025-08-21

### 🐛 Bug Fixes

- Resolve macOS cross-compilation issues in GitHub Actions
- Use native compilation for macOS targets to avoid cross-compilation issues

### ⚙️ Miscellaneous Tasks

- Update rari package version to 0.2.16 and adjust dependencies in pnpm-lock.yaml and package.json
- Adjust resource limits and improve performance
- Bump rari package versions to 0.2.12 for all platforms
- Update GitHub Actions workflow for release process
- Update GitHub Actions workflow for release process
## [rari@0.2.16] - 2025-08-21

### 💼 Other

- Rari@0.2.16

### ⚙️ Miscellaneous Tasks

- Remove CHANGELOG.md and update release script to handle changelog copying
## [0.2.11] - 2025-08-21

### ⚙️ Miscellaneous Tasks

- Add CHANGELOG.md with updates for versions 0.2.10 and 0.2.10-next.0, including features, bug fixes, and miscellaneous tasks
- Update rari package version from 0.2.15-next.0 to 0.2.15 and adjust optional dependencies for all platforms
- Update rari crate version to 0.2.4, adjust dependencies, and improve error handling in module loader
- Bump rari package versions to 0.2.11 for all platforms
## [rari@0.2.15] - 2025-08-20

### 💼 Other

- Rari@0.2.15
## [0.2.10] - 2025-08-20

### ⚙️ Miscellaneous Tasks

- Bump rari package version to 0.2.15-next.0
- Update rari package version to 0.2.15-next.0
- Remove CHANGELOG.md and update release script to copy generated changelog to package directory
- Update rari package versions to 0.2.10 for all platforms
## [0.2.10-next.0] - 2025-08-20

### 🚀 Features

- Integrate rscClient and configure streaming settings

### ⚙️ Miscellaneous Tasks

- Increase resource limits and configuration parameters for improved performance
- Bump binary versions to 0.2.8-next.0
- Bump rari package version to 0.2.3
- Bump rari package versions to 0.2.10-next.0 for all platforms
## [rari@0.2.14] - 2025-08-20

### 🐛 Bug Fixes

- Preserve React module side effects to prevent import transformation issues
- Revert docs config to use correct React plugin
- Remove React OXC plugin to resolve import_react conflict
- Rolling back aggressive optimizations

### 🚜 Refactor

- Clean up Vite config

### ⚙️ Miscellaneous Tasks

- Update optionalDependencies in package.json to version 0.2.2 for Rari binaries
- Bump rari package version to 0.2.3
- Update rari package version to 0.2.3 in package.json
- Update rari package version to 0.2.4 and format Cargo.toml for consistency
- Update rari package version to 0.2.4 in package.json
- Update rari package version to 0.2.12
- Update rari package version to 0.2.13 and bump optional dependencies to 0.2.9
- Update rari package version to 0.2.13 and bump optional dependencies to 0.2.9 for all platforms
- Bump create-rari-app version to 0.1.9
- Bump rari package version to 0.2.14
- Update rari package version to 0.2.14 in pnpm-lock.yaml and package.json
## [0.2.2] - 2025-08-15

### 🚜 Refactor

- Update React imports
- Update ReactDOM usage to createRoot and StrictMode

### ⚙️ Miscellaneous Tasks

- Update rari package version to 0.2.2 and adjust dependencies in pnpm-lock.yaml and docs/package.json
- Update rari package to use workspace link and remove unused cache control settings
- Bump package versions to 0.2.2 for multiple Rari binaries
## [rari@0.2.2] - 2025-08-15

### 💼 Other

- Rari@0.2.2
## [0.2.1] - 2025-08-15

### 🚀 Features

- Enhance documentation and meta description handling
- Enhance caching and HTML optimization in Rari server
- Enhance Google Tag Manager integration and improve layout responsiveness

### 💼 Other

- Framework-level React DOM optimizations - partial implementation

### 🚜 Refactor

- Enhance router functionality and improve code structure
- Streamline Vite configuration and enhance component registration

### ⚙️ Miscellaneous Tasks

- Update rari package version to 0.2.8 in package.json and dependencies
- Bump package versions to 0.2.1 for multiple Rari binaries
## [rari@0.2.7] - 2025-08-16

### ⚙️ Miscellaneous Tasks

- Update rari package versions to 0.2.7 in pnpm-lock.yaml for all platforms
## [0.2.7] - 2025-08-16

### 🚀 Features

- Integrate RSC client and add social icons

### ⚙️ Miscellaneous Tasks

- Update rari package version to 0.2.1 in package.json and pnpm-lock.yaml
- Bump all package versions to 0.2.7 after rollback to stable commit
## [rari@0.2.1] - 2025-08-13

### 💼 Other

- Rari@0.2.1

### 🚜 Refactor

- Streamline RscClient endpoint configuration

### ⚙️ Miscellaneous Tasks

- Update rari package versions in pnpm-lock.yaml to 0.2.0 for all platforms
- Update rari package version to 0.2.0 in package.json and pnpm-lock.yaml
## [create-rari-app@0.1.8] - 2025-08-13

### 💼 Other

- Create-rari-app@0.1.8
## [rari@0.2.0] - 2025-08-13

### 💼 Other

- Rari@0.2.0

### ⚙️ Miscellaneous Tasks

- *(release)* Support RELEASE_VERSION/RELEASE_TYPE env for non-interactive
## [0.2.0] - 2025-08-13

### 🚀 Features

- Enhance Vite configuration for React integration

### 💼 Other

- Bump version to 0.1.7 in create-rari-app package

### 🚜 Refactor

- Enhance error handling and improve streaming functionality
- Streamline error handling and reduce console warnings in init_react.js
- Remove streaming_v2 module and update streaming functionality
- Simplify fallback handling in rendering components
- Streamline module specifier generation and enhance promise resolution handling
- Improve promise handling in StreamingRenderer
- Enhance package selection in release script
- Update dependency registration logic in RscRenderer
- Optimize dependency registration in RscRenderer

### ⚙️ Miscellaneous Tasks

- Update dependencies and improve streaming support
- Update rari package version and dependencies
- Update oxlint and rolldown-vite dependencies across multiple packages
- Update rolldown-vite version to 7.1.2 and enhance streaming functionality
- Update changelog generation settings and remove unused changelog files
- Update esbuild and eslint-react dependencies
- Update GitHub Actions workflow to use Ubuntu 22.04
- *(release)* Platform binaries v0.2.0
## [0.1.5] - 2025-08-07

### 💼 Other

- Rari-linux-x64@0.1.5
- Rari-linux-arm64@0.1.5
- Rari-darwin-x64@0.1.5
- Rari-darwin-arm64@0.1.5
- Rari-win32-x64@0.1.5

### ⚙️ Miscellaneous Tasks

- Update rari dependency versions to 0.1.4 in examples and docs, and bump optional dependencies to 0.1.5 in rari package
## [create-rari-app@0.1.6] - 2025-08-07

### 💼 Other

- Create-rari-app@0.1.6
## [rari@0.1.4] - 2025-08-07

### 💼 Other

- Rari@0.1.4
## [0.1.4] - 2025-08-07

### 🚀 Features

- Add Railway configuration for docs deployment
- Enhance documentation with version fetching & add note on in progress to README
- *(docs)* Add Google Analytics tracking script to index.html

### 🐛 Bug Fixes

- Use published rari package in docs and examples
- Update Railway configuration for proper Node 20 and pnpm setup
- Update start script in package.json to use pnpm exec
- *(create-rari-app)* Resolve .gitignore copy issue and update railway config

### 🚜 Refactor

- Remove Suspense wrapper around Version component in Layout and HomePage

### 📚 Documentation

- Add note on active development and known issues in README

### 🎨 Styling

- Enhance layout responsiveness and improve text wrapping

### ⚙️ Miscellaneous Tasks

- Remove obsolete Railway configuration file
- Update Node version in .nvmrc and modify start script in package.json
- *(create-rari-app)* Bump version to 0.1.5 and update App component
- Add git-cliff configuration and update changelog generation
- Bump all packages to v0.1.4
