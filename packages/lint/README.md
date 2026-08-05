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

| Export              | Purpose                                           |
| ------------------- | ------------------------------------------------- |
| `@rari/lint/vite`   | `fmt`, `lint`, and `ignorePatterns` for vite-plus |
| `@rari/lint/eslint` | Flat ESLint config                                |

## Notes

- Node.js `>=22.18.0`
- Extend with project-specific ignores and overrides as needed

## Links

- **Documentation:** [rari.build/docs](https://rari.build/docs)
- **GitHub:** [github.com/rari-build/rari](https://github.com/rari-build/rari)
- **Discord:** [Join our community](https://discord.gg/GSh2Ak3b8Q)

## License

MIT License - see [LICENSE](https://github.com/rari-build/rari/blob/main/LICENSE) for details.
