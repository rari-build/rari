import { describe, expect, it } from 'vite-plus/test'
import {
  assertExternalsRewritten,
  packageStillRequired,
  rewriteExternalRequires,
} from '../../../tools/bundle-react-esm/rewrite-external-requires'

const EXTERNALIZED_ENTRIES = [
  {
    name: 'react-jsx-runtime',
    externals: { react: 'ext:rari/react/vendor/react.js' },
  },
  {
    name: 'react-dom-server',
    externals: {
      'react': 'ext:rari/react/vendor/react.js',
      'react-dom': 'ext:rari/react/vendor/react-dom.js',
    },
  },
  {
    name: 'react-server-dom-webpack-client',
    externals: {
      'react': 'ext:rari/react/vendor/react.js',
      'react-dom': 'ext:rari/react/vendor/react-dom.js',
    },
  },
  {
    name: 'react-server-dom-webpack-server',
    externals: {
      react: 'ext:rari/react/vendor/react-server.js',
    },
  },
] as const

function minifiedFixture(pkgs: readonly string[]): string {
  const calls = pkgs.map(pkg => `l(\`${pkg}\`)`).join(';')
  return `var l=(e=>typeof require<"u"?require:typeof Proxy<"u"?new Proxy(e,{get:(e,t)=>(typeof require<"u"?require:e)[t]}):e)(()=>{});${calls};export{}`
}

describe('rewriteExternalRequires', () => {
  it.each(EXTERNALIZED_ENTRIES)(
    'rewrites minified helper requires for $name',
    ({ name, externals }) => {
      const pkgs = Object.keys(externals)
      const input = minifiedFixture(pkgs)

      for (const pkg of pkgs) expect(packageStillRequired(input, pkg)).toBe(true)

      const rewritten = rewriteExternalRequires(input, externals)
      assertExternalsRewritten(name, rewritten, externals)

      for (const [pkg, target] of Object.entries(externals)) {
        expect(packageStillRequired(rewritten, pkg)).toBe(false)
        expect(rewritten).toContain(`__ext_${pkg.replace(/\W/g, '_')}`)
        expect(rewritten).toContain(`from '${target}'`)
      }

      expect(rewritten).not.toMatch(/\bl\(([`"'])react\1\)/)
      expect(rewritten).not.toMatch(/\bl\(([`"'])react-dom\1\)/)
    },
  )

  it('rewrites unminified __require calls', () => {
    const externals = { react: 'ext:rari/react/vendor/react.js' }
    const input = 'const React = __require("react"); export default React'
    const rewritten = rewriteExternalRequires(input, externals)

    expect(packageStillRequired(rewritten, 'react')).toBe(false)
    expect(rewritten).toContain("import * as __ext_react from 'ext:rari/react/vendor/react.js'")
    expect(rewritten).toContain('const React = __ext_react')
  })
})
