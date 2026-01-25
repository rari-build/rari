import { join } from 'node:path'
import { antfu } from '@antfu/eslint-config'
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
  ...oxlint.buildFromOxlintConfigFile(join(import.meta.dirname, '.oxlintrc.json')),
)
