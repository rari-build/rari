import { join } from 'node:path'
import { antfu } from '@antfu/eslint-config'
import oxlint from 'eslint-plugin-oxlint'

export default antfu(
  { react: true },
  ...oxlint.buildFromOxlintConfigFile(join(import.meta.dirname, '.oxlintrc.json')),
  {
    rules: {
      'pnpm/json-enforce-catalog': 'off',
    },
  },
  {
    files: ['**/*.md/**'],
    rules: {
      'react-refresh/only-export-components': 'off',
    },
  },
)
