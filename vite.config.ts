import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite-plus'

export default defineConfig({
  resolve: {
    alias: {
      '@rari': fileURLToPath(new URL('./packages/rari/src', import.meta.url)),
    },
  },
  test: {
    globals: true,
    include: ['test/**/*.test.ts'],
    setupFiles: ['./test/setup.ts'],
    coverage: {
      include: [
        'packages/*/src/**/*.{ts,tsx}',
      ],
      exclude: [
        'node_modules',
        'test',
        '**/*.config.ts',
        '**/dist',
      ],
    },
  },
  lint: {
    plugins: [
      'import',
      'unicorn',
    ],
    categories: {
      correctness: 'off',
    },
    env: {
      builtin: true,
      es2026: true,
      browser: true,
      node: true,
    },
    ignorePatterns: [
      '**/target',
      '**/dist',
      '**/node_modules',
      'packages/*/bin',
      '**/.cache',
      '**/coverage',
      '**/test-results',
      '**/playwright-report',
      'playwright/.cache',
      '**/tmp',
      '**/pnpm-lock.yaml',
      '**/CHANGELOG*.md',
      '**/LICENSE*',
      'templates/**',
    ],
    rules: {
      'accessor-pairs': [
        'error',
        {
          enforceForClassMembers: true,
          setWithoutGet: true,
        },
      ],
      'array-callback-return': 'error',
      'block-scoped-var': 'error',
      'constructor-super': 'error',
      'default-case-last': 'error',
      'eqeqeq': [
        'error',
        'smart',
      ],
      'new-cap': [
        'error',
        {
          capIsNew: false,
          newIsCap: true,
          properties: true,
        },
      ],
      'no-alert': 'error',
      'no-array-constructor': 'error',
      'no-async-promise-executor': 'error',
      'no-caller': 'error',
      'no-case-declarations': 'error',
      'no-class-assign': 'error',
      'no-compare-neg-zero': 'error',
      'no-cond-assign': [
        'error',
        'always',
      ],
      'no-console': [
        'error',
        {
          allow: [
            'warn',
            'error',
          ],
        },
      ],
      'no-const-assign': 'error',
      'no-control-regex': 'error',
      'no-debugger': 'error',
      'no-delete-var': 'error',
      'no-dupe-class-members': 'error',
      'no-dupe-keys': 'error',
      'no-duplicate-case': 'error',
      'no-empty': [
        'error',
        {
          allowEmptyCatch: true,
        },
      ],
      'no-empty-pattern': 'error',
      'no-eval': 'error',
      'no-ex-assign': 'error',
      'no-extend-native': 'error',
      'no-extra-bind': 'error',
      'no-extra-boolean-cast': 'error',
      'no-fallthrough': 'error',
      'no-func-assign': 'error',
      'no-global-assign': 'error',
      'no-import-assign': 'error',
      'no-irregular-whitespace': 'error',
      'no-iterator': 'error',
      'no-labels': [
        'error',
        {
          allowLoop: false,
          allowSwitch: false,
        },
      ],
      'no-lone-blocks': 'error',
      'no-loss-of-precision': 'error',
      'no-misleading-character-class': 'error',
      'no-multi-str': 'error',
      'no-new': 'error',
      'no-new-func': 'error',
      'no-new-native-nonconstructor': 'error',
      'no-new-wrappers': 'error',
      'no-obj-calls': 'error',
      'no-proto': 'error',
      'no-prototype-builtins': 'error',
      'no-redeclare': [
        'error',
        {
          builtinGlobals: false,
        },
      ],
      'no-regex-spaces': 'error',
      'no-restricted-globals': [
        'error',
        {
          message: 'Use `globalThis` instead.',
          name: 'global',
        },
        {
          message: 'Use `globalThis` instead.',
          name: 'self',
        },
      ],
      'no-self-assign': [
        'error',
        {
          props: true,
        },
      ],
      'no-self-compare': 'error',
      'no-sequences': 'error',
      'no-shadow-restricted-names': 'error',
      'no-sparse-arrays': 'error',
      'no-template-curly-in-string': 'error',
      'no-this-before-super': 'error',
      'no-throw-literal': 'error',
      'no-undef': 'error',
      'no-unexpected-multiline': 'error',
      'no-unmodified-loop-condition': 'error',
      'no-unneeded-ternary': [
        'error',
        {
          defaultAssignment: false,
        },
      ],
      'no-unreachable': 'error',
      'no-unsafe-finally': 'error',
      'no-unsafe-negation': 'error',
      'no-unused-expressions': [
        'error',
        {
          allowShortCircuit: true,
          allowTaggedTemplates: true,
          allowTernary: true,
        },
      ],
      'no-unused-vars': [
        'error',
        {
          args: 'none',
          caughtErrors: 'none',
          ignoreRestSiblings: true,
          vars: 'all',
        },
      ],
      'no-use-before-define': [
        'error',
        {
          classes: false,
          functions: false,
          variables: true,
        },
      ],
      'no-useless-call': 'error',
      'no-useless-catch': 'error',
      'no-useless-computed-key': 'error',
      'no-useless-constructor': 'error',
      'no-useless-rename': 'error',
      'no-useless-return': 'error',
      'no-var': 'error',
      'no-with': 'error',
      'object-shorthand': [
        'error',
        'always',
        {
          avoidQuotes: true,
          ignoreConstructors: false,
        },
      ],
      'prefer-const': [
        'error',
        {
          destructuring: 'all',
          ignoreReadBeforeAssign: true,
        },
      ],
      'prefer-exponentiation-operator': 'error',
      'prefer-promise-reject-errors': 'error',
      'prefer-rest-params': 'error',
      'prefer-spread': 'error',
      'prefer-template': 'error',
      'symbol-description': 'error',
      'unicode-bom': [
        'error',
        'never',
      ],
      'use-isnan': [
        'error',
        {
          enforceForIndexOf: true,
          enforceForSwitchCase: true,
        },
      ],
      'valid-typeof': [
        'error',
        {
          requireStringLiterals: true,
        },
      ],
      'vars-on-top': 'error',
      'yoda': [
        'error',
        'never',
      ],
      'import/consistent-type-specifier-style': [
        'error',
        'prefer-top-level',
      ],
      'import/first': 'error',
      'import/no-duplicates': 'error',
      'import/no-mutable-exports': 'error',
      'import/no-named-default': 'error',
      'unicorn/consistent-empty-array-spread': 'error',
      'unicorn/error-message': 'error',
      'unicorn/escape-case': 'error',
      'unicorn/new-for-builtins': 'error',
      'unicorn/no-instanceof-builtins': 'error',
      'unicorn/no-new-array': 'error',
      'unicorn/no-new-buffer': 'error',
      'unicorn/number-literal-case': 'error',
      'unicorn/prefer-dom-node-text-content': 'error',
      'unicorn/prefer-includes': 'error',
      'unicorn/prefer-node-protocol': 'error',
      'unicorn/prefer-number-properties': 'error',
      'unicorn/prefer-string-starts-ends-with': 'error',
      'unicorn/prefer-type-error': 'error',
      'unicorn/throw-new-error': 'error',
    },
    overrides: [
      {
        files: [
          '**/*.?([cm])[jt]s?(x)',
        ],
        rules: {
          'node/handle-callback-err': [
            'error',
            '^(err|error)$',
          ],
          'node/no-exports-assign': 'error',
          'node/no-new-require': 'error',
          'node/no-path-concat': 'error',
          'jsdoc/check-access': 'warn',
          'jsdoc/check-property-names': 'warn',
          'jsdoc/empty-tags': 'warn',
          'jsdoc/implements-on-classes': 'warn',
          'jsdoc/no-defaults': 'warn',
          'jsdoc/require-param-name': 'warn',
          'jsdoc/require-property': 'warn',
          'jsdoc/require-property-description': 'warn',
          'jsdoc/require-property-name': 'warn',
          'jsdoc/require-returns-description': 'warn',
        },
        plugins: [
          'node',
          'jsdoc',
        ],
      },
      {
        files: [
          '**/*.?([cm])ts',
          '**/*.?([cm])tsx',
        ],
        rules: {
          'constructor-super': 'off',
          'getter-return': 'off',
          'no-class-assign': 'off',
          'no-const-assign': 'off',
          'no-dupe-keys': 'off',
          'no-func-assign': 'off',
          'no-import-assign': 'off',
          'no-new-native-nonconstructor': 'off',
          'no-obj-calls': 'off',
          'no-redeclare': [
            'error',
            {
              builtinGlobals: false,
            },
          ],
          'no-setter-return': 'off',
          'no-this-before-super': 'off',
          'no-undef': 'off',
          'no-unreachable': 'off',
          'no-unsafe-negation': 'off',
          'no-with': 'off',
          'prefer-const': [
            'error',
            {
              destructuring: 'all',
              ignoreReadBeforeAssign: true,
            },
          ],
          'no-unused-expressions': [
            'error',
            {
              allowShortCircuit: true,
              allowTaggedTemplates: true,
              allowTernary: true,
            },
          ],
          'no-unused-vars': 'off',
          'no-useless-constructor': 'off',
          'no-use-before-define': [
            'error',
            {
              classes: false,
              functions: false,
              variables: true,
            },
          ],
          'typescript/ban-ts-comment': [
            'error',
            {
              'ts-expect-error': 'allow-with-description',
            },
          ],
          'typescript/no-duplicate-enum-values': 'error',
          'typescript/no-dynamic-delete': 'off',
          'typescript/no-empty-object-type': [
            'error',
            {
              allowInterfaces: 'always',
            },
          ],
          'typescript/no-explicit-any': 'off',
          'typescript/no-extra-non-null-assertion': 'error',
          'typescript/no-extraneous-class': 'off',
          'typescript/no-invalid-void-type': 'off',
          'typescript/no-misused-new': 'error',
          'typescript/no-namespace': 'error',
          'typescript/no-non-null-asserted-nullish-coalescing': 'error',
          'typescript/no-non-null-asserted-optional-chain': 'error',
          'typescript/no-non-null-assertion': 'off',
          'typescript/no-require-imports': 'error',
          'typescript/no-this-alias': 'error',
          'typescript/no-unnecessary-type-constraint': 'error',
          'typescript/no-unsafe-declaration-merging': 'error',
          'typescript/no-unsafe-function-type': 'error',
          'typescript/no-wrapper-object-types': 'error',
          'typescript/prefer-as-const': 'error',
          'typescript/prefer-literal-enum-member': 'error',
          'typescript/prefer-namespace-keyword': 'error',
          'typescript/triple-slash-reference': 'off',
          'typescript/unified-signatures': 'off',
          'typescript/consistent-type-definitions': [
            'error',
            'interface',
          ],
          'typescript/consistent-type-imports': [
            'error',
            {
              disallowTypeAnnotations: false,
              fixStyle: 'separate-type-imports',
              prefer: 'type-imports',
            },
          ],
          'typescript/no-import-type-side-effects': 'error',
        },
        plugins: [
          'typescript',
        ],
      },
      {
        files: [
          '**/__tests__/**/*.?([cm])[jt]s?(x)',
          '**/*.spec.?([cm])[jt]s?(x)',
          '**/*.test.?([cm])[jt]s?(x)',
          '**/*.bench.?([cm])[jt]s?(x)',
          '**/*.benchmark.?([cm])[jt]s?(x)',
        ],
        rules: {
          'vitest/consistent-test-it': [
            'error',
            {
              fn: 'it',
              withinDescribe: 'it',
            },
          ],
          'vitest/no-identical-title': 'error',
          'vitest/no-import-node-test': 'error',
          'vitest/prefer-hooks-in-order': 'error',
          'vitest/prefer-lowercase-title': 'error',
          'no-unused-expressions': 'off',
          'typescript/explicit-function-return-type': 'off',
        },
        plugins: [
          'vitest',
          'typescript',
        ],
      },
      {
        files: [
          '**/*.?([cm])[jt]s?(x)',
        ],
        rules: {
          'react/exhaustive-deps': 'warn',
          'react/no-array-index-key': 'warn',
          'react/no-clone-element': 'warn',
          'react/no-direct-mutation-state': 'error',
          'react/rules-of-hooks': 'error',
          'react/jsx-no-comment-textnodes': 'warn',
          'react/only-export-components': [
            'error',
            {
              allowConstantExport: false,
              allowExportNames: [],
            },
          ],
        },
        plugins: [
          'react',
        ],
      },
      {
        files: [
          '**/*.md/**/*.?([cm])[jt]s?(x)',
        ],
        rules: {
          'no-alert': 'off',
          'no-console': 'off',
          'no-labels': 'off',
          'no-lone-blocks': 'off',
          'no-undef': 'off',
          'no-unused-expressions': 'off',
          'no-unused-labels': 'off',
          'no-unused-vars': 'off',
          'unicode-bom': 'off',
          'no-redeclare': 'off',
          'no-use-before-define': 'off',
          'typescript/consistent-type-imports': 'off',
          'typescript/explicit-function-return-type': 'off',
          'typescript/no-namespace': 'off',
          'typescript/no-require-imports': 'off',
        },
        plugins: [
          'typescript',
        ],
      },
      {
        files: [
          '**/scripts/**/*.?([cm])[jt]s?(x)',
        ],
        rules: {
          'no-console': 'off',
          'typescript/explicit-function-return-type': 'off',
        },
        plugins: [
          'typescript',
        ],
      },
      {
        files: [
          '**/cli/**/*.?([cm])[jt]s?(x)',
          '**/cli.?([cm])[jt]s?(x)',
        ],
        rules: {
          'no-console': 'off',
        },
      },
      {
        files: [
          '**/*.js',
          '**/*.cjs',
        ],
        rules: {
          'typescript/no-require-imports': 'off',
        },
        plugins: [
          'typescript',
        ],
      },
      {
        files: [
          '**/*.config.?([cm])[jt]s?(x)',
          '**/*.config.*.?([cm])[jt]s?(x)',
        ],
        rules: {
          'no-console': 'off',
          'typescript/explicit-function-return-type': 'off',
        },
        plugins: [
          'typescript',
        ],
      },
      {
        files: [
          'examples/**/src/app/**',
          'test/fixtures/**/src/app/**',
          'web/src/app/**',
        ],
        rules: {
          'react/only-export-components': 'off',
        },
        plugins: [
          'react',
        ],
      },
    ],
    options: {
      typeAware: true,
      typeCheck: true,
    },
  },
})
