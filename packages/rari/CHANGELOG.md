## [rari@0.10.0] - 2026-02-18

### 🚀 Features

- *(og)* Add SVG rendering support for open graph images

### 🐛 Bug Fixes

- *(og)* Improve SVG rendering precision and component handling
- *(og)* Handle async components and rendering errors gracefully
- *(og)* Return null for non-function component resolution

### 🚜 Refactor

- *(og)* Extract component resolution logic into dedicated method
## [rari@0.9.3] - 2026-02-17

### 💼 Other

- Rari@0.9.3

### 🚜 Refactor

- Extract regex patterns to constants for reusability
- Optimize regex patterns and simplify conditional logic
- Improve regex patterns and fix capture group references
- Improve path handling and rename regex constants for clarity
- Optimize path resolution and regex pattern ordering
- Improve regex patterns and parameter handling
- Improve parameter extraction and remove unused path alias

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.9.3
## [rari@0.9.2] - 2026-02-14

### 🚀 Features

- *(rsc)* Fix 404 not-found route handling with streaming support

### 💼 Other

- Rari@0.9.2

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.9.2
## [rari@0.9.1] - 2026-02-13

### 🚀 Features

- *(rari)* Implement fetch caching with request deduplication

### 💼 Other

- Rari@0.9.1

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.9.1
## [rari@0.9.0] - 2026-02-12

### 🚀 Features

- *(runtime)* Split client components from main bundle
- *(vite)* Add import graph tracking for client-only component detection

### 🐛 Bug Fixes

- *(runtime)* Improve error handling in file operations and component loading

### 💼 Other

- Rari@0.9.0

### 🚜 Refactor

- *(runtime)* Extract component loading logic and improve client component resolution
- *(runtime)* Unify component loading promise handling
- *(runtime)* Improve component loading and path normalization

### 🎨 Styling

- Format conditional statements and update linting rules
## [rari@0.8.14] - 2026-02-05

### 💼 Other

- Rari@0.8.14

### ⚙️ Miscellaneous Tasks

- *(rari)* Add RariResponse to tsdown exports
## [rari@0.8.13] - 2026-02-05

### 💼 Other

- Rari@0.8.13

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.8.12
## [rari@0.8.12] - 2026-02-04

### 🐛 Bug Fixes

- *(proxy)* Improve module path resolution and runtime imports

### 💼 Other

- Rari@0.8.12
## [rari@0.8.11] - 2026-02-04

### 💼 Other

- Rari@0.8.11
## [0.8.10] - 2026-02-04

### 🚀 Features

- *(runtime)* Export RSC client runtime modules and migrate to TypeScript
- *(vite)* Add wildcard export support and rewrite runtime imports
- *(runtime)* Add react-dom types and migrate to virtual RSC client module

### 🐛 Bug Fixes

- Improve error handling and request header management
- *(runtime)* Improve HTML escaping and RSC element parsing
- *(runtime)* Remove unnecessary non-null assertions and improve line splitting
- *(vite)* Add trailing comma to import replacement regex
- *(vite)* Normalize virtual module imports with file extensions
- *(runtime)* Improve error message handling in app rendering
- *(runtime)* Improve attribute handling and import path resolution
- *(vite)* Simplify import rewriting and improve virtual module resolution
- *(vite)* Improve import rewriting with comprehensive pattern matching
- *(vite)* Add runtime directory resolution for chunk imports
- *(vite)* Improve error handling and add security validation for file resolution
- *(vite)* Improve import path matching to support optional parent directory references
- *(vite)* Add rari package detection for .mjs file resolution
- *(vite)* Improve import path matching for react-server-dom-rari-client
- *(router)* Move sitemap directory creation to after module validation
- *(vite)* Skip TypeScript declaration files during directory scan

### 🚜 Refactor

- Extract logger utilities into shared package
- Improve cross-platform compatibility and code clarity
- Simplify code and improve test mocking patterns
- Improve type safety and path normalization
- *(proxy)* Extract path normalization logic and add csrf retry test
- *(runtime)* Replace global type declarations with accessor functions
- *(runtime)* Extract getClientComponent to shared utility
- *(runtime)* Extract global type definitions to shared types module
- *(runtime)* Remove unused client component registry and utilities

### ⚙️ Miscellaneous Tasks

- *(runtime)* Remove react-server-dom-shim type definitions
- *(package)* Remove src directory from published files
## [rari@0.8.10] - 2026-02-03

### 🚀 Features

- *(rsc-renderer)* Improve error handling and RSC serialization format

### 🐛 Bug Fixes

- *(vite)* Remove redundant component path normalization

### 💼 Other

- Rari@0.8.10

### 🚜 Refactor

- *(rsc-renderer)* Optimize RSC rendering and improve path handling

### 🧪 Testing

- Add comprehensive unit test suite with vitest

### ⚙️ Miscellaneous Tasks

- *(packages)* Remove @rari/colors package and migrate to native Node.js utilities
## [rari@0.8.9] - 2026-02-02

### 💼 Other

- Rari@0.8.9
## [rari@0.8.8] - 2026-02-02

### 🐛 Bug Fixes

- *(vite)* Handle absolute paths in client and server component resolution

### 💼 Other

- Rari@0.8.8
## [rari@0.8.7] - 2026-02-02

### 💼 Other

- Rari@0.8.7
## [rari@0.8.6] - 2026-02-02

### 🚀 Features

- *(module_loader)* Improve CommonJS detection and require handling

### 🐛 Bug Fixes

- Improve error handling and module type detection across build pipeline
- *(vite)* Improve export parsing and rolldown output handling
- *(router)* Disable code splitting in robots generator build config
- *(vite)* Remove unnecessary resolveDir and external config from server build
- *(router)* Disable file writing in build configs for generators
- *(module_loader,router)* Improve path resolution and build output handling
- *(router)* Ensure output directory exists and improve robots file extension handling
- *(router)* Improve type safety and module type handling in robots generator

### 💼 Other

- Rari@0.8.6

### 🚜 Refactor

- *(router)* Improve variable naming in robots generator

### ⚙️ Miscellaneous Tasks

- Migrate remaining uses of esbuild to rolldown
- *(release)* Remove generated changelog and improve release tooling
## [rari@0.8.5] - 2026-02-01

### 🚀 Features

- *(deploy)* Extract deployment utilities into standalone package

### 💼 Other

- Rari@0.8.5
- Rari@0.8.5
- Rari@0.8.5

### 🚜 Refactor

- *(rari)* Simplify tsdown configuration and external dependencies
- *(packages)* Standardize tsconfig includes and improve deploy exports

### ⚙️ Miscellaneous Tasks

- *(release)* Add file generation for README and LICENSE in packages
- *(rari)* Bump binary package versions to 0.8.5
- *(rari)* Bump version to 0.8.4
- *(rari)* Revert version to 0.8.4
## [rari@0.8.4] - 2026-01-30

### 🐛 Bug Fixes

- Correct typos and improve error handling in RSC rendering
- Correct error handling and redirect URL assignment

### 💼 Other

- Rari@0.8.4

### 🚜 Refactor

- Extract React component and SSR manifest creation logic
- *(vite)* Extract error messages to variables for clarity
- *(vite)* Remove unused component tracking sets

### 🎨 Styling

- Add blank lines for improved code readability
- Remove unnecessary braces from single-statement conditionals
- Simplify conditional expressions and improve code readability
- Improve regex patterns for string and path normalization
- Remove unused variables and simplify property deletion
- Remove unused variable assignments and simplify encoding defaults
- *(cli)* Remove unnecessary blank lines in detectPackageManager
## [rari@0.8.3] - 2026-01-30

### 💼 Other

- Rari@0.8.3
## [rari@0.8.2] - 2026-01-30

### 🚀 Features

- *(image)* Add quality allowlist configuration for image optimization
- *(image)* Add preoptimization manifest support for image variants
- *(image)* Add preload image tracking and metadata injection support
- *(image)* Improve image scanner and optimizer robustness
- *(image)* Improve image scanner and optimizer robustness

### 💼 Other

- Rari@0.8.2

### ⚙️ Miscellaneous Tasks

- *(build)* Update Node.js target version to 22
- *(rari)* Bump optional dependencies to 0.8.2
## [rari@0.8.1] - 2026-01-29

### 🚀 Features

- *(vite)* Expand optimizeDeps configuration for React core modules
- *(image)* Add CLI subcommand for pre-optimizing local images
- *(rari)* Export Metadata type and add type annotations to metadata exports
- *(image)* Add dry-run mode to image optimization CLI and improve type safety
- *(image)* Add rkyv serialization for image cache and improve async file operations
- *(cli)* Add automatic package manager detection and cross-platform execution
- *(cli)* Improve package manager detection with monorepo support

### 💼 Other

- Rari@0.8.1

### 🚜 Refactor

- *(rari)* Improve path normalization and proxy configuration
- *(rari)* Separate server config from manifest and rename types
- Improve code quality and type safety across codebase
- Improve async handling and optimize image processing pipeline
- Modernize code patterns and improve image optimization configuration
- Optimize image processing and improve code constants
- *(image)* Optimize cache operations and improve async file handling
- Migrate to ES modules and improve cross-platform compatibility
- *(deployment)* Extract shared utilities and improve Node version validation
- *(logger)* Extract logging utilities into dedicated module
- *(logger)* Remove deprecated logWarning function in favor of logWarn
- *(deployment)* Improve Node version parsing with multiple format support

### 🎨 Styling

- Modernize JavaScript and TypeScript code patterns
- Remove unnecessary braces from single-statement conditionals

### ⚙️ Miscellaneous Tasks

- Upgrade Node.js minimum version to 20.6.0
- *(rari)* Bump optional dependencies to 0.8.1
## [rari@0.8.0] - 2026-01-27

### 🚀 Features

- *(csp)* Add worker-src directive support
- *(vite)* Add HTML import detection and build optimization
- *(sitemap)* Add dynamic sitemap generation support
- *(sentry)* Implement dynamic import and optimize bundle splitting

### 🐛 Bug Fixes

- *(router)* Simplify server URL resolution in ClientRouter

### 💼 Other

- Rari@0.8.0

### 🚜 Refactor

- *(colors)* Extract colors utility into standalone package

### ⚙️ Miscellaneous Tasks

- *(rari)* Update optional dependencies to 0.8.0
## [rari@0.7.14] - 2026-01-27

### 🐛 Bug Fixes

- *(vite)* Correct client reference property names

### 💼 Other

- Rari@0.7.14
## [rari@0.7.13] - 2026-01-27

### 🚀 Features

- *(vite)* Improve server build module resolution and client component handling

### 💼 Other

- Rari@0.7.13

### 🚜 Refactor

- *(vite)* Rename server build plugin for clarity
## [rari@0.7.12] - 2026-01-27

### 🚀 Features

- *(vite)* Enhance client component scanning and import resolution
- *(vite)* Enhance server-side component and action reference handling

### 🐛 Bug Fixes

- *(vite)* Update component import regex to support alias paths

### 💼 Other

- Rari@0.7.12
## [rari@0.7.11] - 2026-01-26

### 🚀 Features

- *(rari)* Add platform entry point to tsdown
- *(rari)* Add input options to suppress postcss export warnings

### 💼 Other

- Rari@0.7.11

### 🚜 Refactor

- *(router)* Replace chokidar with vite dev server watcher
- *(router)* Replace dynamic import function with vite-ignore comments
## [rari@0.7.10] - 2026-01-24

### 💼 Other

- *(tsdown)* Enable minification for build output
- Rari@0.7.10
## [rari@0.7.9] - 2026-01-24

### 💼 Other

- Rari@0.7.9

### 🚜 Refactor

- *(vite)* Replace acorn with esbuild
- *(rari)* Replace picocolors with internal colors util
## [rari@0.7.8] - 2026-01-23

### 💼 Other

- Rari@0.7.8
## [rari@0.7.7] - 2026-01-23

### 🚀 Features

- *(rendering)* Fix support for dangerouslySetInnerHTML and object-style props
- *(cli)* Add dev command and support scoped package names
- *(cli)* Add build command and unify package scripts
- *(deployment)* Migrate Railway configuration from JSON to TOML format

### 💼 Other

- Rari@0.7.7
## [rari@0.7.6] - 2026-01-21

### 🚀 Features

- *(rari)* Refactor routing and rendering architecture with improved component resolution
- *(rari,web)* Add custom define option and integrate sponsor link into navigation
- *(og)* Add ImageResponse documentation

### 💼 Other

- Rari@0.7.5
- Rari@0.7.6

### 🚜 Refactor

- *(exports)* Reorganize proxy and image module exports

### 🎨 Styling

- Standardize rari branding to lowercase in console messages
- Standardize rari branding to lowercase in error messages
- Standardize rari branding in log messages and error outputs

### ⚙️ Miscellaneous Tasks

- *(lint)* Consolidate eslint configuration and update tooling
- *(build)* Consolidate workspace dependencies and enhance clippy linting
- *(rari)* Bump optional dependencies to 0.7.5
- *(release)* Fix package.json and improve version handling
## [rari@0.7.4] - 2026-01-15

### 🚀 Features

- *(rari)* Optimize component resolution and rendering performance

### 💼 Other

- Rari@0.7.4

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump binary version to 0.7.4
## [rari@0.7.3] - 2026-01-14

### 🚀 Features

- *(rari)* Enhance open graph and twitter metadata handling

### 💼 Other

- Rari@0.7.3

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump binary version to 0.7.3
## [rari@0.7.2] - 2026-01-13

### 💼 Other

- Rari@0.7.2
## [rari@0.7.1] - 2026-01-13

### 💼 Other

- Rari@0.7.1

### ⚙️ Miscellaneous Tasks

- Remove dav1d dependency and update dependencies
- *(rari)* Bump optional dependencies to 0.7.1
## [rari@0.7.0] - 2026-01-13

### 🚀 Features

- *(image)* Add image optimization and caching system
- *(og)* Add open graph image generation with dynamic rendering
- *(image)* Add local image pattern matching and AVIF native support
- *(image)* Change default image format from WebP to AVIF
- *(security)* Make CSRF protection optional with environment configuration

### 🐛 Bug Fixes

- *(rsc)* Handle stale content and missing promises gracefully

### 💼 Other

- Rari@0.7.0

### 🚜 Refactor

- *(hmr)* Consolidate HMR handlers into unified action endpoint
- *(rsc)* Stream RSC responses directly without buffering
- *(vite)* Fix client component HMR handling
- *(image)* Rename priority to preload and add custom loader support
- *(api)* Consolidate internal routes under /_rari namespace
- *(config)* Migrate CSP and rate limit config from environment variables to manifest

### ⚙️ Miscellaneous Tasks

- *(router)* Remove item from skip directories list
- *(rari)* Bump optional dependencies to 0.7.0
## [rari@0.6.1] - 2026-01-08

### 💼 Other

- Rari@0.6.1
## [rari@0.6.0] - 2026-01-08

### 🚀 Features

- *(docs)* Add package manager tabs and terminal blocks to MDX
- *(proxy)* Add request/response proxy middleware and runtime execution
- *(robots)* Add robots.txt generation support
- *(metadata)* Add comprehensive metadata support for icons, theme, and apple web app
- *(vite)* Skip robots and sitemap files in server component scanning

### 💼 Other

- Rari@0.6.0

### 🚜 Refactor

- *(router)* Remove loading component map generation

### 🎨 Styling

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

### ⚙️ Miscellaneous Tasks

- *(rari)* Remove useActionState hook and exports
- *(rari)* Remove file extensions from mdx exports
- *(rari)* Remove AppRouterProvider export from package.json
- *(rari)* Remove fsevents from external dependencies
- *(vite)* Remove external dependencies configuration
- *(build)* Reorganize server manifests into dist/server directory
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
## [rari@0.5.29] - 2026-01-02

### 🚀 Features

- *(rsc)* Improve streaming updates
- *(rsc)* Enhance lazy loading and streaming completion handling
- *(rsc-client-runtime)* Improve RSC row parsing
- *(rsc)* Optimize lazy loading and promise resolution handling
- *(rari)* Implement partial hydration and dynamic module loading
- *(rsc)* Implement lazy promise resolution and streaming suspense

### 🐛 Bug Fixes

- *(rari)* Remove debug console.warn statements from RSC client

### 💼 Other

- Rari@0.5.29
## [rari@0.5.28] - 2025-12-24

### 🐛 Bug Fixes

- *(server)* Return correct HTTP status codes for not-found routes

### 💼 Other

- Rari@0.5.28
## [rari@0.5.27] - 2025-12-24

### 💼 Other

- Rari@0.5.27
## [rari@0.5.26] - 2025-12-23

### 🚀 Features

- *(routing)* Implement dynamic route info endpoint and remove manifest injection

### 💼 Other

- Rari@0.5.26
## [rari@0.5.25] - 2025-12-23

### 🚀 Features

- *(rsc)* Add RSC wire format payload and manifest embedding
- *(mdx)* Replace mdx-remote with native @mdx-js/mdx compilation

### 🐛 Bug Fixes

- *(rsc)* Correct React symbol property names from single to double dollar signs

### 💼 Other

- Rari@0.5.25
## [rari@0.5.24] - 2025-12-20

### 💼 Other

- Rari@0.5.24

### 🚜 Refactor

- *(rsc)* Rename client component registry globals to use tilde prefix
- *(rsc)* Migrate global namespace from __rari to ~rari
- *(rsc)* Migrate global namespace from __rari to ~rari
- *(rsc)* Migrate global namespace from __rsc to ~rsc
- *(rsc)* Migrate global namespace from double underscore to tilde prefix
- *(hmr)* Improve component specifier handling and remove debug logging
- *(runtime)* Remove lifecycle logging from LayoutWrapper

### ⚙️ Miscellaneous Tasks

- *(logging)* Remove warn-level logging statements
- *(logging)* Remove warn-level logging and upgrade to error-level where appropriate
## [rari@0.5.23] - 2025-12-18

### 💼 Other

- Rari@0.5.23

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.5.17
## [rari@0.5.22] - 2025-12-17

### 💼 Other

- Rari@0.5.22

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.5.16
## [rari@0.5.21] - 2025-12-17

### 💼 Other

- Rari@0.5.21

### 🚜 Refactor

- *(rsc)* Standardize boundary ID prop naming to ~boundaryId
- *(rsc)* Standardize client component marker prop naming
- *(rsc)* Standardize data attribute naming with tilde prefix

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.5.15
## [rari@0.5.20] - 2025-12-17

### 🚀 Features

- *(runtime)* Add module loading and component initialization improvements
- *(rari)* Add external module configuration for server builds

### 💼 Other

- Rari@0.5.20

### 🚜 Refactor

- *(vite)* Remove unnecessary banners and optimize minification settings
- *(vite)* Simplify node imports transformation in server build

### ⚙️ Miscellaneous Tasks

- *(release)* Bump rari to 0.5.14
## [rari@0.5.19] - 2025-12-16

### 🚀 Features

- *(docs)* Add dynamic metadata generation and route path conversion

### 💼 Other

- Rari@0.5.19
## [rari@0.5.18] - 2025-12-16

### 💼 Other

- Rari@0.5.18
## [rari@0.5.17] - 2025-12-16

### 🚀 Features

- *(metadata)* Add page metadata collection and injection system

### 💼 Other

- Rari@0.5.17
## [rari@0.5.16] - 2025-12-13

### 💼 Other

- Rari@0.5.16

### 🚜 Refactor

- *(docs,examples)* Migrate to path aliases
## [rari@0.5.15] - 2025-12-12

### 🚀 Features

- *(runtime)* Enhance Node.js compatibility stubs for fs, path, and process

### 🐛 Bug Fixes

- *(router)* Improve component loading fallback logic

### 💼 Other

- Rari@0.5.15

### 🚜 Refactor

- *(docs,rari)* Improve markdown rendering and module resolution
## [rari@0.5.14] - 2025-12-11

### 💼 Other

- Rari@0.5.14
## [rari@0.5.13] - 2025-12-11

### 🐛 Bug Fixes

- *(cli)* Guard main execution to prevent running when imported as module

### 💼 Other

- Rari@0.5.13

### ⚙️ Miscellaneous Tasks

- *(server)* Simplify startup logging and remove verbose debug output
- *(server)* Remove verbose startup completion messages
- *(release)* Bump rari platform dependencies to 0.5.8
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
## [rari@0.5.10] - 2025-12-09

### 💼 Other

- Rari@0.5.10

### ⚙️ Miscellaneous Tasks

- *(rari)* Bump optional dependencies to 0.5.6
## [0.5.6] - 2025-12-09

### 🚀 Features

- *(server)* Integrate CSRF token generation into RSC rendering pipeline
- *(csrf)* Move CSRF token generation to client-side runtime

### ⚙️ Miscellaneous Tasks

- Upgrade dependencies and enhance linting rules
## [rari@0.5.9] - 2025-12-09

### 💼 Other

- Rari@0.5.9

### ⚙️ Miscellaneous Tasks

- *(release)* Bump binaries to 0.5.5
## [rari@0.5.8] - 2025-12-04

### 🚀 Features

- *(rsc)* Extract client-side RSC runtime from inline and update rendering pipeline
- *(runtime)* Extract client-side entry point and RSC wire format parser

### 💼 Other

- Rari@0.5.8

### 🎨 Styling

- *(runtime)* Clean up linting directives and modernize string formatting

### ⚙️ Miscellaneous Tasks

- *(release)* Bump binaries to 0.5.4
## [rari@0.5.7] - 2025-12-04

### 💼 Other

- Rari@0.5.7
## [rari@0.5.6] - 2025-12-03

### 🐛 Bug Fixes

- *(router)* Add Vite ignore comments to dynamic imports

### 💼 Other

- Rari@0.5.6
## [rari@0.5.5] - 2025-12-02

### 💼 Other

- Rari@0.5.5
## [0.5.1] - 2025-12-02

### 🚜 Refactor

- *(rsc)* Replace timestamp-based key generation with counter for stable key gen
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

### 🚀 Features

- *(router)* Implement client-side routing with layout management and error handling

### 💼 Other

- Rari@0.5.0

### 🚜 Refactor

- *(vite)* Simplify client-side rendering logic
- *(exports)* Rename server export to vite for clarity
## [rari@0.4.0] - 2025-10-30

### 🚀 Features

- *(runtime)* Add API request bridge for Rari server runtime

### 💼 Other

- Rari@0.4.0
## [rari@0.3.3] - 2025-10-28

### 💼 Other

- Rari@0.3.3
## [rari@0.3.2] - 2025-10-28

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
- *(rsc)* Introduce direct HTML rendering and enhance layout rendering with caching

### 🐛 Bug Fixes

- Correct component path replacement

### 🚜 Refactor

- Remove 'use server' directives from components and update documentation to clarify server/client component usage
- Replace entry-client.tsx with virtual import for client components
- Simplify server component detection logic by removing redundant checks
- Streamline rari router plugin by removing unused options and simplifying route generation logic
- Clean up routing code by deleting obsolete files
- Remove obsolete '.rari' directory references and streamline routing logic
- Enhance middleware handling and dynamic Vite port config
- Expanding HMR for app router support
- *(AppRouterHMRProvider)* Streamline event handling and enhance RSC payload management
- *(ssr)* Reorganize SSR rendering logic and improve error handling
- *(rsc)* Remove renderer pool and simplify layout rendering
- *(rari)* Simplify server component checks and remove unused directory validations
- *(rsc)* Simplify caching logic and remove route-specific cache exclusions
- *(rsc)* Rename RSC function registration global to improve clarity
- *(rsc)* Simplify component registration and remove redundant hash-based keys
- *(rsc)* Improve component invalidation and registration mechanisms
- *(rsc)* Improve component import transformation for global component resolution
- *(rsc)* Improve client-side rendering and hydration logic
- *(rsc)* Fix hydration logic in renderApp
- *(server)* Enhance static file handling and security configuration

### ⚙️ Miscellaneous Tasks

- Update server and Vite configuration ports to 5173
- *(linting)* Update ESLint and Oxlint configuration
- *(build)* Update build output directory from .rari to dist
- *(dependencies)* Update project dependencies to latest versions
- *(dependencies)* Update project dependencies
- Fix linting errors
- *(dependencies)* Update project dependencies and linting config
- *(rari)* Bump package version to 0.3.0
## [rari@0.2.24] - 2025-09-23

### 💼 Other

- Rari@0.2.24

### ⚙️ Miscellaneous Tasks

- Update dependencies across multiple packages to latest versions
## [rari@0.2.23] - 2025-09-23

### 🚀 Features

- Add support for component initial loading state and enhance HMR functionality

### 💼 Other

- Rari@0.2.23
## [rari@0.2.22] - 2025-09-04

### 💼 Other

- Rari@0.2.22

### 🚜 Refactor

- Improve formatting and error handling in extractCacheConfigFromContent function
## [rari@0.2.21] - 2025-09-04

### 💼 Other

- Rari@0.2.21

### ⚙️ Miscellaneous Tasks

- Update rari dependencies to version 0.2.15 and remove unused client-dev entry
## [rari@0.2.20] - 2025-09-04

### 💼 Other

- Rari@0.2.20

### 🚜 Refactor

- Remove unused router components and functions, simplify package.json dependencies

### ⚙️ Miscellaneous Tasks

- Update dependencies across multiple packages to latest versions
## [rari@0.2.19] - 2025-09-02

### 💼 Other

- Rari@0.2.19

### ⚙️ Miscellaneous Tasks

- Update rari to version 0.2.7, add caching configuration support, and remove macOS Rust version override in release workflow
- Update optionalDependencies for rari to version 0.2.14
## [rari@0.2.18] - 2025-08-22

### 💼 Other

- Rari@0.2.18

### ⚙️ Miscellaneous Tasks

- Update dependencies
## [rari@0.2.17] - 2025-08-21

### 💼 Other

- Rari@0.2.17
## [rari@0.2.16] - 2025-08-21

### 💼 Other

- Rari@0.2.16
## [rari@0.2.15] - 2025-08-20

### 🐛 Bug Fixes

- Preserve React module side effects to prevent import transformation issues
- Rolling back aggressive optimizations

### 💼 Other

- Rari@0.2.15

### ⚙️ Miscellaneous Tasks

- Update rari package to use workspace link and remove unused cache control settings
- Update optionalDependencies in package.json to version 0.2.2 for Rari binaries
- Bump rari package version to 0.2.3
- Update rari package version to 0.2.4 and format Cargo.toml for consistency
- Update rari package version to 0.2.13 and bump optional dependencies to 0.2.9
- Bump rari package version to 0.2.14
- Bump rari package version to 0.2.15-next.0
- Remove CHANGELOG.md and update release script to copy generated changelog to package directory
## [rari@0.2.2] - 2025-08-15

### 🚀 Features

- Enhance documentation and meta description handling
- Enhance caching and HTML optimization in Rari server
- Enhance Google Tag Manager integration and improve layout responsiveness

### 💼 Other

- Framework-level React DOM optimizations - partial implementation
- Rari@0.2.2

### 🚜 Refactor

- Enhance router functionality and improve code structure
- Streamline Vite configuration and enhance component registration

### ⚙️ Miscellaneous Tasks

- Update rari package version to 0.2.8 in package.json and dependencies
## [0.2.7] - 2025-08-16

### ⚙️ Miscellaneous Tasks

- Bump all package versions to 0.2.7 after rollback to stable commit
## [rari@0.2.1] - 2025-08-13

### 💼 Other

- Rari@0.2.1

### 🚜 Refactor

- Streamline RscClient endpoint configuration
## [rari@0.2.0] - 2025-08-13

### 🚀 Features

- Enhance Vite configuration for React integration

### 💼 Other

- Rari@0.2.0

### 🚜 Refactor

- Enhance error handling and improve streaming functionality
- Remove streaming_v2 module and update streaming functionality
- Simplify fallback handling in rendering components
- Streamline module specifier generation and enhance promise resolution handling

### ⚙️ Miscellaneous Tasks

- Update dependencies and improve streaming support
- Update oxlint and rolldown-vite dependencies across multiple packages
- Update rolldown-vite version to 7.1.2 and enhance streaming functionality
- Update changelog generation settings and remove unused changelog files
- Update esbuild and eslint-react dependencies
## [0.1.5] - 2025-08-07

### ⚙️ Miscellaneous Tasks

- Update rari dependency versions to 0.1.4 in examples and docs, and bump optional dependencies to 0.1.5 in rari package
## [rari@0.1.4] - 2025-08-07

### 💼 Other

- Rari@0.1.4
