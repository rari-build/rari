import { join } from 'node:path'
import { antfu } from '@antfu/eslint-config'
import oxlint from 'eslint-plugin-oxlint'

export default antfu(
  { react: true },
  ...oxlint.buildFromOxlintConfigFile(join(import.meta.dirname, '.oxlintrc.json')),
  {
    files: ['README.md', '**/*.md/**'],
    rules: {
      'react-dom/no-dangerously-set-innerhtml': 'off',
      'react-refresh/only-export-components': 'off',
      'style/jsx-one-expression-per-line': 'off',
    },
  },
  {
    files: ['docs/src/app/**', 'examples/**/src/app/**', 'packages/create-rari-app/templates/**/src/app/**'],
    rules: {
      'react-refresh/only-export-components': 'off',
    },
  },
)
