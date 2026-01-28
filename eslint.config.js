import { join } from 'node:path'
import { antfu } from '@antfu/eslint-config'
import e18e from '@e18e/eslint-plugin'
import oxlint from 'eslint-plugin-oxlint'

export default antfu(
  {
    react: true,
    typescript: true,
    ignores: [
      '**/packages/create-rari-app/templates/**',
    ],
  },
  {
    files: ['examples/**/src/app/**', 'web/src/app/**'],
    rules: {
      'react-refresh/only-export-components': 'off',
    },
  },
  e18e.configs.recommended,
  ...oxlint.buildFromOxlintConfigFile(join(import.meta.dirname, '.oxlintrc.json')),
)
