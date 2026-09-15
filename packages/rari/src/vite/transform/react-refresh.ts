import type { Plugin } from 'vite-plus'
import { reactRefreshWrapperPlugin } from 'vite-plus/internal'
import { REACT_REFRESH_RUNTIME_CODE } from './react-refresh-runtime'

export const REACT_REFRESH_PATH = '/@react-refresh'

const INCLUDE_RE = /\.[cm]?[jt]sx?$/
const EXCLUDE = [/node_modules/, /\/dist\//]

export function getReactRefreshPreambleScript(refreshModuleUrl: string): string {
  return `import { injectIntoGlobalHook } from '${refreshModuleUrl}'
injectIntoGlobalHook(window)
window.$RefreshReg$ = () => {}
window.$RefreshSig$ = () => type => type
window.__vite_plugin_react_preamble_installed__ = true`
}

export function createReactRefreshPlugins(): Plugin[] {
  const runtimePlugin: Plugin = {
    name: 'rari:react-refresh-runtime',
    enforce: 'pre',
    apply: 'serve',
    resolveId(id) {
      if (id === REACT_REFRESH_PATH) return id
      return null
    },
    load(id) {
      if (id === REACT_REFRESH_PATH) return REACT_REFRESH_RUNTIME_CODE
      return null
    },
  }

  const wrapperPlugin: Plugin = {
    name: 'rari:react-refresh-wrapper',
    apply: 'serve',
    applyToEnvironment(env) {
      if (env.config.consumer !== 'client') return false

      return reactRefreshWrapperPlugin({
        cwd: process.cwd(),
        include: [INCLUDE_RE],
        exclude: EXCLUDE,
        jsxImportSource: 'react',
        reactRefreshHost: '',
      })
    },
  }

  return [runtimePlugin, wrapperPlugin]
}
