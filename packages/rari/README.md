# rari

Runtime Accelerated Rendering Infrastructure — a Rust host for React Server Components and other frameworks.

This package is the `rari` CLI (`rari dev`, `rari build`, `rari start`, `rari deploy`) and the npm distribution of the Rust server binary. It also re-exports the React adapter, so `import { … } from 'rari'` and every `rari/*` subpath keep working; the React implementation lives in [`@rari/react`](../react), the framework-agnostic build contracts in [`@rari/core`](../core), and the Qwik adapter in [`@rari/qwik`](../qwik).

Which guest framework the server loads is recorded by the framework's build in `dist/server/config.json` (`"framework": "react" | "qwik"`), and can be overridden with `rari --framework <name>` or `RARI_FRAMEWORK`.

See the [rari documentation](https://rari.build).
