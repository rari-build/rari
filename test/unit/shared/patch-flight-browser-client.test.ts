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
  it('injects form-action helpers with page URL resolver from the edge client', () => {
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
    expect(patched).toContain('action: resolveRariFormActionUrl()')
    expect(patched).toContain('$$FORM_ACTION')
    expect(patched).not.toContain('function registerBoundServerReference(reference, id, bound) {')
  })

  it('throws when the browser anchor is missing instead of silently no-oping', () => {
    const edgeSource = fs.readFileSync(
      resolveReactCjs('react-server-dom-webpack', 'react-server-dom-webpack-client.edge'),
      'utf-8',
    )

    expect(() => patchBrowserClientForFormActions('const unrelated = 1;', edgeSource)).toThrow(
      /browser registerBoundServerReference not found/,
    )
  })

  it('throws when the browser anchor matches more than once', () => {
    const browserSource = fs.readFileSync(
      resolveReactCjs('react-server-dom-webpack', 'react-server-dom-webpack-client.browser'),
      'utf-8',
    )
    const edgeSource = fs.readFileSync(
      resolveReactCjs('react-server-dom-webpack', 'react-server-dom-webpack-client.edge'),
      'utf-8',
    )

    expect(() =>
      patchBrowserClientForFormActions(browserSource + browserSource, edgeSource),
    ).toThrow(/browser registerBoundServerReference matched more than once/)
  })

  it('throws when the edge form-action return block is missing', () => {
    const fakeEdgeSource = [
      'var boundCache = new WeakMap();',
      'var somethingElse = 1;',
      'function createBoundServerReference() {}',
    ].join('\n')

    expect(() => patchBrowserClientForFormActions('irrelevant', fakeEdgeSource)).toThrow(
      /edge \$\$FORM_ACTION return block not found/,
    )
  })

  it('throws when the edge form-action return block matches more than once', () => {
    const returnBlock = `return {
    name: referenceClosure,
    method: "POST",
    encType: "multipart/form-data",
    data: data
  };`
    const fakeEdgeSource = [
      'var boundCache = new WeakMap();',
      returnBlock,
      returnBlock,
      'function createBoundServerReference() {}',
    ].join('\n')

    expect(() => patchBrowserClientForFormActions('irrelevant', fakeEdgeSource)).toThrow(
      /edge \$\$FORM_ACTION return block matched more than once/,
    )
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
