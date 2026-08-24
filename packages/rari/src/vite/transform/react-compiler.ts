/* oxlint-disable typescript/prefer-readonly-parameter-types -- oxc ReactCompilerOptions is a mutable options bag */
import type { ReactCompilerOptions as OxcReactCompilerOptions } from 'oxc-transform-react'
import type { Plugin } from 'vite-plus'

export type ReactCompilerOptions = OxcReactCompilerOptions
export type RariCompilerOption = boolean | ReactCompilerOptions

export const REACT_COMPILER_CODE_FILTER = /forwardRef|memo|\b(?:[A-Z]|use[A-Z0-9])/

const DEFAULT_INCLUDE_RE = /\.[cm]?[jt]sx?$/
const QUERY_STRIP_RE = /\?.*$/
const USE_MEMO_DIRECTIVE_RE = /['"]use memo['"]/

type OxcTransformReact = typeof import('oxc-transform-react')

function resolveCompilerOptions(
  compiler: Exclude<RariCompilerOption, false>,
): ReactCompilerOptions {
  return compiler === true ? {} : compiler
}

export function shouldApplyReactCompiler(code: string, options: ReactCompilerOptions): boolean {
  if (options.compilationMode === 'annotation') return USE_MEMO_DIRECTIVE_RE.test(code)
  return REACT_COMPILER_CODE_FILTER.test(code)
}

export function compilerRuntimeModule(options: ReactCompilerOptions): string {
  const target = options.target
  return target === '17' || target === '18' ? 'react-compiler-runtime' : 'react/compiler-runtime'
}

export function matchesCompilerId(id: string): boolean {
  const cleanId = id.replace(QUERY_STRIP_RE, '')
  return DEFAULT_INCLUDE_RE.test(cleanId) && !cleanId.includes('/node_modules/')
}

export function createReactCompilerPlugin(compiler: Exclude<RariCompilerOption, false>): Plugin {
  const options = resolveCompilerOptions(compiler)
  let oxc: OxcTransformReact | undefined
  let sourcemap = true
  let jsxDevelopment = false
  let fastRefresh = false

  const loadOxc = async (onError: (message: string) => never): Promise<OxcTransformReact> => {
    if (oxc) return oxc
    try {
      oxc = await import('oxc-transform-react')
      return oxc
    } catch (error) {
      return onError(
        `React Compiler requires the optional \`oxc-transform-react\` package. Install it before enabling \`rari({ compiler: true })\`.${
          error instanceof Error ? `\n${error.message}` : ''
        }`,
      )
    }
  }

  return {
    name: 'rari:react-compiler',
    enforce: 'pre',
    async config(_, { command }) {
      await loadOxc(message => this.error(message))
      fastRefresh = command === 'serve'
      return {
        // Own Fast Refresh when the compiler plugin runs so Vite's oxc
        // refresh pass does not double-register.
        oxc: {
          jsx: {
            refresh: false,
          },
        },
        optimizeDeps: {
          include: [compilerRuntimeModule(options)],
        },
      }
    },
    configResolved(config) {
      sourcemap = config.command !== 'build' || config.build.sourcemap !== false
      jsxDevelopment = !config.isProduction
      fastRefresh =
        !config.isProduction && config.command === 'serve' && config.server.hmr !== false
    },
    async transform(code, id) {
      if (!matchesCompilerId(id)) return null

      const isClient = this.environment.config.consumer !== 'server'
      // Compiler + Fast Refresh are client-only; leave RSC/SSR to Vite oxc.
      if (!isClient) return null

      const { transform } = oxc ?? (await loadOxc(message => this.error(message)))
      const filename = id.replace(QUERY_STRIP_RE, '')
      const shouldCompile = shouldApplyReactCompiler(code, options)

      const result = await transform(filename, code, {
        jsx: {
          runtime: 'automatic',
          development: jsxDevelopment,
          importSource: 'react',
          refresh: fastRefresh,
        },
        reactCompiler: shouldCompile ? options : false,
        sourcemap,
      })

      const diagnostics = result.errors.map(error => {
        const codeframe = error.codeframe
        if (codeframe != null && codeframe !== '') return `${error.message}\n${codeframe}`
        return error.message
      })

      if (result.fatal) {
        const message = diagnostics.join('\n\n')
        this.error(message !== '' ? message : 'React Compiler transform failed.')
      }
      for (const diagnostic of diagnostics) {
        this.warn(diagnostic)
      }

      return { code: result.code, map: result.map }
    },
  }
}
