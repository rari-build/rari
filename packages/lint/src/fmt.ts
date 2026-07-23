import type { OxfmtConfig } from 'vite-plus/fmt'
import { ignorePatterns } from './ignores.ts'

export const fmt: OxfmtConfig = {
  semi: false,
  singleQuote: true,
  jsxSingleQuote: false,
  trailingComma: 'all',
  arrowParens: 'avoid',
  bracketSpacing: true,
  quoteProps: 'consistent',
  endOfLine: 'lf',
  insertFinalNewline: true,
  tabWidth: 2,
  sortPackageJson: false,
  sortImports: {
    order: 'asc',
    newlinesBetween: false,
    groups: [
      'type-import',
      ['type-parent', 'type-sibling', 'type-index', 'type-internal'],
      'value-builtin',
      'value-external',
      'value-internal',
      ['value-parent', 'value-sibling', 'value-index'],
      'side_effect',
      'unknown',
    ],
  },
  ignorePatterns: [...ignorePatterns, '**/*.mdx'],
  overrides: [
    {
      files: ['**/*.{json,jsonc}'],
      options: {
        trailingComma: 'none',
      },
    },
  ],
}
