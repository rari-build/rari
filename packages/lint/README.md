# @rari/lint

> Lint and format configs for the [rari](https://github.com/rari-build/rari) framework

## Installation

```bash
npm install -D @rari/lint eslint vite-plus
```

Install `vite-plus` when using the Vite export.

## Usage

### vite-plus (`fmt` + `lint`)

```ts
import { fmt, lint } from '@rari/lint/vite'
import { defineConfig } from 'vite-plus'

export default defineConfig({
  fmt,
  lint,
})
```

### ESLint flat config

The ESLint config only covers files oxlint doesn't lint: pnpm catalog and settings checks plus key sorting for `package.json` and `pnpm-workspace.yaml`, and key sorting for TypeScript config files (`tsconfig*.json`, `jsconfig*.json`). Other JSON and YAML files aren't checked. All JavaScript, TypeScript, and React rules (including `@eslint-react`) run in oxlint through `@rari/lint/vite`, so use both exports together.

```ts
// eslint.config.ts
export { default } from '@rari/lint/eslint'
```

Or extend and override:

```ts
import rariLint from '@rari/lint/eslint'

export default [
  ...rariLint,
  {
    rules: {
      // project overrides
    },
  },
]
```

## Exports

| Export              | Purpose                                                            |
| ------------------- | ------------------------------------------------------------------ |
| `@rari/lint/vite`   | `fmt`, `lint`, and `ignorePatterns` for vite-plus                  |
| `@rari/lint/eslint` | Flat ESLint config for package.json, tsconfig, pnpm-workspace.yaml |

## Notes

- Node.js `^24.21.0 || >=26.0.0`
- Extend with project-specific ignores and overrides as needed

## Links

- **Documentation:** [rari.build/docs](https://rari.build/docs)
- **GitHub:** [github.com/rari-build/rari](https://github.com/rari-build/rari)
- **Discord:** [Join our community](https://discord.gg/GSh2Ak3b8Q)

## License

MIT License - see [LICENSE](https://github.com/rari-build/rari/blob/main/LICENSE) for details.
