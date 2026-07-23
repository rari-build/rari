import type { ESLint, Linter } from 'eslint'
import jsoncPlugin from 'eslint-plugin-jsonc'
import * as jsoncParser from 'jsonc-eslint-parser'

// oxlint-disable-next-line typescript/no-unsafe-type-assertion
const pluginJsonc = jsoncPlugin as ESLint.Plugin

export const packageJsonSortConfigs: Linter.Config[] = [
  {
    name: 'rari/sort/package-json',
    files: ['package.json', '**/package.json'],
    languageOptions: {
      parser: jsoncParser,
    },
    plugins: {
      jsonc: pluginJsonc,
    },
    rules: {
      'jsonc/sort-array-values': [
        'error',
        {
          order: { type: 'asc' },
          pathPattern: '^files$',
        },
      ],
      'jsonc/sort-keys': [
        'error',
        {
          order: [
            'publisher',
            'name',
            'displayName',
            'type',
            'version',
            'private',
            'packageManager',
            'description',
            'author',
            'contributors',
            'license',
            'funding',
            'homepage',
            'repository',
            'bugs',
            'keywords',
            'categories',
            'sideEffects',
            'imports',
            'exports',
            'main',
            'module',
            'unpkg',
            'jsdelivr',
            'types',
            'typesVersions',
            'bin',
            'icon',
            'files',
            'engines',
            'activationEvents',
            'contributes',
            'scripts',
            'scripts-info',
            'peerDependencies',
            'peerDependenciesMeta',
            'dependencies',
            'optionalDependencies',
            'devDependencies',
            'pnpm',
            'overrides',
            'resolutions',
            'husky',
            'simple-git-hooks',
            'lint-staged',
            'eslintConfig',
          ],
          pathPattern: '^$',
        },
        {
          order: { type: 'asc' },
          pathPattern: '^(?:dev|peer|optional|bundled)?[Dd]ependencies(Meta)?$',
        },
        {
          order: { type: 'asc' },
          pathPattern: '^(?:resolutions|overrides|pnpm.overrides)$',
        },
        {
          order: { type: 'asc' },
          pathPattern: '^workspaces\\.catalog$',
        },
        {
          order: { type: 'asc' },
          pathPattern: '^workspaces\\.catalogs\\.[^.]+$',
        },
        {
          order: ['types', 'import', 'node', 'require', 'default'],
          pathPattern: '^exports.*$',
        },
        {
          order: [
            'pre-commit',
            'prepare-commit-msg',
            'commit-msg',
            'post-commit',
            'pre-rebase',
            'post-rewrite',
            'post-checkout',
            'post-merge',
            'pre-push',
            'pre-auto-gc',
          ],
          pathPattern: '^(?:gitHooks|husky|simple-git-hooks)$',
        },
      ],
    },
  },
]
