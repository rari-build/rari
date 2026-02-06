import { antfu } from '@antfu/eslint-config'
import e18e from '@e18e/eslint-plugin'
import deMorgan from 'eslint-plugin-de-morgan'
import oxlint from 'eslint-plugin-oxlint'

export default antfu(
  {
    react: true,
    typescript: true,
    ignores: ['**/packages/create-rari-app/templates/**'],
  },
  {
    rules: {
      'padding-line-between-statements': [
        'error',
        { blankLine: 'always', prev: ['if', 'for', 'while', 'switch'], next: 'return' },
        { blankLine: 'always', prev: 'block-like', next: 'return' },
      ],
    },
  },
  {
    files: ['examples/**/src/app/**', 'web/src/app/**'],
    rules: {
      'react-refresh/only-export-components': 'off',
    },
  },
  // TODO: Remove this once @antfu/eslint-config fixes the upstream issue
  // Related: https://github.com/antfu/eslint-config/pull/808
  {
    rules: {
      'react/no-implicit-key': 'off',
    },
  },
  deMorgan.configs.recommended,
  e18e.configs.recommended,
  ...oxlint.buildFromOxlintConfigFile(`${import.meta.dirname}/.oxlintrc.json`),
)
