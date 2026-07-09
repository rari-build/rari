import fs from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import {
  fixRolldownDoubleDollarProperties,
  patchBrowserClientForFormActions,
} from '@rari/shared/patch-flight-browser-client'
import { describe, expect, it } from 'vite-plus/test'

const require = createRequire(import.meta.url)

function resolveReactCjs(pkg: string, cjsFile: string): string {
  const pkgJson = require.resolve(`${pkg}/package.json`)
  const pkgDir = path.dirname(pkgJson)
  return path.join(pkgDir, 'cjs', `${cjsFile}.production.js`)
}

describe('patchBrowserClientForFormActions', () => {
  it('injects form-action helpers with /_rari/action from the edge client', () => {
    const browserSource = fs.readFileSync(
      resolveReactCjs('react-server-dom-webpack', 'react-server-dom-webpack-client.browser'),
      'utf-8',
    )
    const edgeSource = fs.readFileSync(
      resolveReactCjs('react-server-dom-webpack', 'react-server-dom-webpack-client.edge'),
      'utf-8',
    )

    const patched = fixRolldownDoubleDollarProperties(
      patchBrowserClientForFormActions(browserSource, edgeSource),
    )

    expect(patched).toContain('var boundCache = new WeakMap();')
    expect(patched).toContain('action: "/_rari/action"')
    expect(patched).toContain('$$FORM_ACTION')
    expect(patched).not.toContain('function registerBoundServerReference(reference, id, bound) {')
  })
})

describe('fixRolldownDoubleDollarProperties', () => {
  it('restores double-dollar React internal property names', () => {
    const input = 'props.$FORM_ACTION = fn; props.$IS_SIGNATURE_EQUAL = eq;'
    expect(fixRolldownDoubleDollarProperties(input)).toBe(
      'props.$$FORM_ACTION = fn; props.$$IS_SIGNATURE_EQUAL = eq;',
    )
  })
})
