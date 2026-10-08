# @rari/react

The React adapter for [rari](https://rari.build): React Server Components, the app router, server actions, images, fonts, MDX and the Vite plugin, rendered by rari's Rust host.

This is the implementation behind the `rari` package. Existing apps keep importing from `rari` (`rari`, `rari/router`, `rari/vite`, …), which re-exports every subpath of `@rari/react`; new code may import `@rari/react` directly. The `rari` package itself is the CLI and the npm distribution of the Rust binary.

Shared, framework-independent pieces (route manifest schema and scanner, metadata conventions, binary resolution) live in `@rari/core`.

See the [rari documentation](https://rari.build) for usage.
