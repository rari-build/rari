import type { Linter } from 'eslint'
import type { OxfmtConfig } from 'vite-plus/fmt'
import type { OxlintConfig } from 'vite-plus/lint'
import { fmt as rariFmt, lint as rariLint } from '@rari/lint/vite'

export const monorepoIgnorePatterns: string[] = [
  'packages/*/bin',
  'packages/rari-*/',
  'packages/use-cache-*/',
  'packages/create-rari-app/templates/**',
]

export const monorepoReadonlyParameterAllows: Array<
  string | { from: 'lib'; name: string[] } | { from: 'package'; name: string[]; package: string }
> = [
  {
    from: 'lib',
    name: [
      'URL',
      'AbortSignal',
      'Error',
      'TypeError',
      'RegExp',
      'Date',
      'Uint8Array',
      'Event',
      'Element',
      'HTMLElement',
      'HTMLButtonElement',
      'KeyboardEvent',
      'PopStateEvent',
      'WheelEvent',
      'PageTransitionEvent',
    ],
  },
  {
    from: 'package',
    name: [
      'ReactElement',
      'ReactPortal',
      'SyntheticEvent',
      'MouseEvent',
      'CSSProperties',
      'ErrorInfo',
      'SVGProps',
    ],
    package: 'react',
  },
  { from: 'package', name: ['PostHog'], package: 'posthog-js' },
  { from: 'package', name: ['Buffer'], package: 'node' },
  { from: 'package', name: ['UserConfig'], package: 'vite-plus' },
  { from: 'package', name: ['RolldownOutput'], package: 'rolldown' },
  { from: 'package', name: ['ThemeRegistration'], package: '@shikijs/types' },
  { from: 'package', name: ['PageProps', 'LayoutProps'], package: 'rari' },
  { from: 'package', name: ['EvaluateOptions'], package: '@mdx-js/mdx' },
  'Page',
  'APIRequestContext',
  'ViteDevServer',
  'Plugin',
  'ModuleNode',
  'HmrContext',
  'RariRequest',
  'EvaluateOptions',
  'CachedFetchInit',
  'PathLike',
  'SpawnOptions',
  'ChildProcess',
  'ResponseInit',
  'RequestInit',
  'PropertyDescriptorMap',
  'Addr',
  'CommandOptions',
  'Conn',
  'ConnectOptions',
  'ConnectTlsOptions',
  'CreateHttpClientOptions',
  'ForeignFunction',
  'KvCheck',
  'KvListSelector',
  'ListenDatagramOptions',
  'ListenOptions',
  'ListenTlsOptions',
  'MakeTempOptions',
  'MkdirOptions',
  'OpenOptions',
  'ReadFileOptions',
  'RemoveOptions',
  'ResolveDnsOptions',
  'ServeOptions',
  'StartTlsOptions',
  'SymlinkOptions',
  'UnsafeCallbackDefinition',
  'UpgradeWebSocketOptions',
  'WriteFileOptions',
]

export const monorepoEslintConfigs: Linter.Config[] = [
  {
    ignores: monorepoIgnorePatterns,
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
]

export const monorepoFmt: OxfmtConfig = {
  ...rariFmt,
  ignorePatterns: [...(rariFmt.ignorePatterns ?? []), ...monorepoIgnorePatterns],
}

export const monorepoLint: OxlintConfig = {
  ...rariLint,
  ignorePatterns: [...(rariLint.ignorePatterns ?? []), ...monorepoIgnorePatterns],
  rules: {
    ...rariLint.rules,
    'typescript/prefer-readonly-parameter-types': [
      'error',
      {
        ignoreInferredTypes: true,
        treatMethodsAsReadonly: true,
        allow: monorepoReadonlyParameterAllows,
      },
    ],
  },
  overrides: [
    ...(rariLint.overrides ?? []),
    {
      files: ['tools/bundle-react-esm/*.ts'],
      rules: {
        'no-console': 'off',
      },
    },
    {
      files: [
        'crates/rari/src/runtime/ext/**/*.{ts,tsx,d.ts}',
        'crates/rari/src/rendering/**/*.{ts,tsx,d.ts}',
      ],
      rules: {
        'no-undef': 'off',
        'no-unused-vars': 'off',
        'no-use-before-define': 'off',
        'typescript/ban-ts-comment': 'off',
        'antfu/no-top-level-await': 'off',
        'unused-imports/no-unused-imports': 'off',
        'unused-imports/no-unused-vars': 'off',
      },
      plugins: ['typescript'],
    },
  ],
}
