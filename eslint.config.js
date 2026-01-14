import { join } from 'node:path'
import { antfu } from '@antfu/eslint-config'
import oxlint from 'eslint-plugin-oxlint'

export default antfu(
  { react: true },
  ...oxlint.buildFromOxlintConfigFile(join(import.meta.dirname, '.oxlintrc.json')),
  {
    files: ['examples/**/src/app/**', 'packages/create-rari-app/templates/**/src/app/**', 'web/src/app/**'],
    rules: {
      'react-refresh/only-export-components': 'off',
    },
  },
)
