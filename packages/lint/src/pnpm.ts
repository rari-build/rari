import type { ESLint, Linter } from 'eslint'
import pluginPnpm from 'eslint-plugin-pnpm'
import ymlPlugin from 'eslint-plugin-yml'
import * as jsoncParser from 'jsonc-eslint-parser'
import * as yamlParser from 'yaml-eslint-parser'

// oxlint-disable-next-line typescript/no-unsafe-type-assertion
const pluginYaml = ymlPlugin as ESLint.Plugin

export const pnpmConfigs: Linter.Config[] = [
  {
    name: 'rari/pnpm/package-json',
    files: ['package.json', '**/package.json'],
    languageOptions: {
      parser: jsoncParser,
    },
    plugins: {
      pnpm: pluginPnpm,
    },
    rules: {
      'pnpm/json-enforce-catalog': [
        'error',
        {
          autofix: true,
          ignores: ['@types/vscode'],
        },
      ],
      'pnpm/json-prefer-workspace-settings': ['error', { autofix: true }],
      'pnpm/json-valid-catalog': ['error', { autofix: true }],
    },
  },
  {
    name: 'rari/pnpm/pnpm-workspace-yaml',
    files: ['pnpm-workspace.yaml'],
    languageOptions: {
      parser: yamlParser,
    },
    plugins: {
      pnpm: pluginPnpm,
    },
    rules: {
      'pnpm/yaml-enforce-settings': [
        'error',
        {
          settings: {
            shellEmulator: true,
            trustPolicy: 'no-downgrade',
          },
        },
      ],
      'pnpm/yaml-no-duplicate-catalog-item': 'error',
      'pnpm/yaml-no-unused-catalog-item': 'error',
    },
  },
  {
    name: 'rari/pnpm/pnpm-workspace-yaml-sort',
    files: ['pnpm-workspace.yaml'],
    languageOptions: {
      parser: yamlParser,
    },
    plugins: {
      yaml: pluginYaml,
    },
    rules: {
      'yaml/sort-keys': [
        'error',
        {
          order: [
            'cacheDir',
            'catalogMode',
            'cleanupUnusedCatalogs',
            'dedupeDirectDeps',
            'deployAllFiles',
            'enablePrePostScripts',
            'engineStrict',
            'extendNodePath',
            'hoist',
            'hoistPattern',
            'hoistWorkspacePackages',
            'ignoreCompatibilityDb',
            'ignoreDepScripts',
            'ignoreScripts',
            'ignoreWorkspaceRootCheck',
            'managePackageManagerVersions',
            'minimumReleaseAge',
            'minimumReleaseAgeExclude',
            'modulesDir',
            'nodeLinker',
            'nodeVersion',
            'optimisticRepeatInstall',
            'packageManagerStrict',
            'packageManagerStrictVersion',
            'preferSymlinkedExecutables',
            'preferWorkspacePackages',
            'publicHoistPattern',
            'registrySupportsTimeField',
            'requiredScripts',
            'resolutionMode',
            'savePrefix',
            'scriptShell',
            'shamefullyHoist',
            'shellEmulator',
            'stateDir',
            'supportedArchitectures',
            'symlink',
            'tag',
            'trustPolicy',
            'trustPolicyExclude',
            'updateNotifier',
            'packages',
            'overrides',
            'patchedDependencies',
            'catalog',
            'catalogs',
            'allowBuilds',
            'allowedDeprecatedVersions',
            'allowNonAppliedPatches',
            'configDependencies',
            'ignoredBuiltDependencies',
            'ignoredOptionalDependencies',
            'neverBuiltDependencies',
            'onlyBuiltDependencies',
            'onlyBuiltDependenciesFile',
            'packageExtensions',
            'peerDependencyRules',
          ],
          pathPattern: '^$',
        },
        {
          order: { type: 'asc' },
          pathPattern: '.*',
        },
      ],
    },
  },
]
