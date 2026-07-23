## [@rari/use-cache@0.15.5] - 2026-07-23

### 🚀 Features

- wire JS pool into streaming and cut Suspense stream latency by @skiniks

### ⚙️ Miscellaneous Tasks

- *(dependencies)* update various package versions by @skiniks
- *(use-cache)* update optional dependencies to version 0.15.5 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/@rari/use-cache@0.15.4...@rari/use-cache@0.15.5
## [@rari/use-cache@0.15.3] - 2026-07-15

### ⚙️ Miscellaneous Tasks

- update optional dependencies to version 0.15.3 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.15.2...@rari/use-cache@0.15.3
## [0.15.2] - 2026-07-14

### 🚜 Refactor

- introduce TransformError enum for improved error handling in transform module by @skiniks

### ⚙️ Miscellaneous Tasks

- update optional dependencies to version 0.15.2 for all platforms in package.json and pnpm-lock.yaml by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/v0.15.1...v0.15.2
## [0.15.1] - 2026-07-13

### ⚙️ Miscellaneous Tasks

- *(dependencies)* upgrade @rari/use-cache packages to version 0.15.1 by @skiniks


**Full Changelog**: https://github.com/rari-build/rari/compare/@rari/use-cache@0.15.0...v0.15.1
## [@rari/use-cache@0.15.0] - 2026-07-13

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
