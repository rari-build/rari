import type { Linter } from 'eslint'
import react from '@eslint-react/eslint-plugin'
import * as tsParser from '@typescript-eslint/parser'
import gitignore from 'eslint-config-flat-gitignore'
import oxlint from 'eslint-plugin-oxlint'
import reactRefresh from 'eslint-plugin-react-refresh'
import { ignorePatterns } from './ignores.ts'
import { lint as oxlintConfig } from './oxlint.ts'

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
function withReactPrefix(config: Linter.Config): Linter.Config {
  const plugin = config.plugins?.['@eslint-react']
  if (plugin == null || config.rules == null) return config

  const rules: Linter.RulesRecord = {}
  for (const [name, value] of Object.entries(config.rules)) {
    if (value == null) continue
    rules[
      name.startsWith('@eslint-react/') ? `react/${name.slice('@eslint-react/'.length)}` : name
    ] = value
  }

  return {
    ...config,
    name: config.name?.replace('@eslint-react/', 'react/') ?? 'react/recommended-typescript',
    plugins: { react: plugin },
    rules,
  }
}

const oxlintBridge = oxlint
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion
  .buildFromOxlintConfig(oxlintConfig as Parameters<typeof oxlint.buildFromOxlintConfig>[0])
  .map(config => {
    if (!config.rules) return config

    const rules = { ...config.rules }
    for (const [name, value] of Object.entries(config.rules)) {
      if (name.startsWith('@typescript-eslint/'))
        rules[`ts/${name.slice('@typescript-eslint/'.length)}`] = value
      if (name.startsWith('n/')) rules[`node/${name.slice(2)}`] = value
      if (name.startsWith('@eslint-react/'))
        rules[`react/${name.slice('@eslint-react/'.length)}`] = value
    }

    return {
      ...config,
      rules,
    }
  })

const recommendedTypescript = withReactPrefix(react.configs['recommended-typescript'])

export default [
  gitignore(),
  {
    ignores: [...ignorePatterns, '**/packages/create-rari-app/templates/**'],
  },
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaFeatures: { jsx: true },
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
    },
  },
  {
    ...recommendedTypescript,
    files: ['**/*.{ts,tsx}'],
  },
  {
    files: ['crates/rari/src/runtime/ext/**/*.ts'],
    rules: {
      'react/no-unnecessary-use-prefix': 'off',
    },
  },
  {
    files: ['tools/bundle-react-esm/*.ts'],
    rules: {
      'no-console': 'off',
    },
  },
  ...oxlintBridge,
  // After the oxlint bridge so HMR export checks stay enabled (oxlint owns react/only-export-components).
  {
    ...reactRefresh.configs.vite,
    files: ['**/*.{ts,tsx}'],
  },
  {
    files: ['examples/**/src/app/**', 'test/fixtures/**/src/app/**', 'web/src/app/**'],
    rules: {
      'react-refresh/only-export-components': 'off',
    },
  },
] satisfies Linter.Config[]
