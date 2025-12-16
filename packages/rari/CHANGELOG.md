## [0.5.17] - 2025-12-16

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
- *(example)* Enhance blog and interactive pages with improved styling
- *(rsc)* Enhance layout renderer test cases with not_found support

### 📚 Documentation

- *(readme)* Update performance metrics and project overview
- *(readme)* Update project structure and example code
- *(rari)* Add comprehensive package metadata for Cargo.toml

### 🎨 Styling

- *(not-found)* Update 404 page styling and color scheme
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
