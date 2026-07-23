import type { Plugin } from 'vite-plus'
import fs from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import { rari } from '@rari/vite'
import { afterAll, beforeAll, describe, expect, it } from 'vite-plus/test'
import { castMock } from '../helpers/mock-cast'

const FIXTURE_DIR = path.join(process.cwd(), 'test/fixtures/use-cache')

interface UseCacheTransformResult {
  code: string
  needsReactCache: boolean
  needsCacheWrapper: boolean
  needsRegisterRef: boolean
}

interface UseCacheAddon {
  detectUseCache: (source: string) => boolean
  transformUseCache: (
    source: string,
    opts: Readonly<{ filename: string; hashSalt: string }>,
  ) => UseCacheTransformResult
  flushLlvmProfile?: () => void
}

function isUseCacheAddon(value: unknown): value is UseCacheAddon {
  return (
    typeof value === 'object' &&
    value !== null &&
    'detectUseCache' in value &&
    typeof value.detectUseCache === 'function' &&
    'transformUseCache' in value &&
    typeof value.transformUseCache === 'function'
  )
}

let useCacheAddon: UseCacheAddon | null = null

try {
  const repoRoot = process.cwd()
  const ext =
    process.platform === 'win32' ? '.dll' : process.platform === 'darwin' ? '.dylib' : '.so'
  const platform = `${process.platform}-${process.arch}`

  const candidates = [
    path.join(repoRoot, '.build/rari_use_cache', platform, 'rari_use_cache.node'),
    path.join(repoRoot, 'packages', `use-cache-${platform}`, 'rari_use_cache.node'),
    path.join(repoRoot, 'target/debug/rari_use_cache.node'),
    path.join(repoRoot, `target/debug/librari_use_cache${ext}`),
    path.join(repoRoot, 'target/release/rari_use_cache.node'),
    path.join(repoRoot, `target/release/librari_use_cache${ext}`),
  ]

  for (const addonPath of candidates) {
    if (fs.existsSync(addonPath)) {
      const nodeRequire = createRequire(import.meta.url)
      const loaded: unknown = nodeRequire(addonPath)
      if (isUseCacheAddon(loaded)) {
        useCacheAddon = loaded
        break
      }
    }
  }
} catch {
  // addon not available
}

afterAll(() => {
  useCacheAddon?.flushLlvmProfile?.()
})

function writeFixture(name: string, content: string): string {
  const dir = path.join(FIXTURE_DIR, 'src')
  fs.mkdirSync(dir, { recursive: true })
  const filePath = path.join(dir, name)
  fs.writeFileSync(filePath, content, 'utf-8')
  return filePath
}

function cleanFixtures() {
  if (fs.existsSync(FIXTURE_DIR)) {
    fs.rmSync(FIXTURE_DIR, { recursive: true, force: true })
  }
}

function fixturePath(name: string): string {
  return path.join(FIXTURE_DIR, 'src', name)
}

// ── Direct native addon tests (skipped in CI without Rust build) ──

const addonDescribe = useCacheAddon ? describe : describe.skip

addonDescribe('use-cache addon', () => {
  const addon = useCacheAddon!

  describe('detectUseCache', () => {
    it('returns true for double-quoted use cache directive', () => {
      expect(addon.detectUseCache('"use cache";')).toBe(true)
    })

    it('returns true for single-quoted use cache directive', () => {
      expect(addon.detectUseCache("'use cache';")).toBe(true)
    })

    it('returns true for inline use cache directive', () => {
      expect(addon.detectUseCache('"use cache"')).toBe(true)
    })

    it('returns false for plain code', () => {
      expect(addon.detectUseCache('const x = 1;')).toBe(false)
    })

    it('returns false for empty string', () => {
      expect(addon.detectUseCache('')).toBe(false)
    })

    it('returns false for similar-looking strings', () => {
      expect(addon.detectUseCache('"use cached"')).toBe(false)
      expect(addon.detectUseCache('"cache"')).toBe(false)
    })

    it('detects backtick cache directives with custom kinds', () => {
      expect(addon.detectUseCache('`use cache: stale-while-revalidate`')).toBe(true)
      expect(addon.detectUseCache('`use cache:`')).toBe(false)
    })
  })

  describe('transformUseCache', () => {
    const defaultOpts = {
      filename: 'test.js',
      hashSalt: 'rari-use-cache-v1',
    }

    it('transforms async function with use cache directive', () => {
      const src = `
async function getData(id) {
  "use cache";
  return await db.query(id);
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toBe(src)
      expect(result.needsReactCache).toBe(false)
      expect(result.needsCacheWrapper).toBe(true)
      expect(result.needsRegisterRef).toBe(true)
      expect(result.code).not.toContain('$$reactCache__')
      expect(result.code).toContain('$$cache__')
      expect(result.code).toContain('registerServerReference')
      expect(result.code).toContain('$$RSC_SERVER_CACHE_0_getData_INNER')
      expect(result.code).toContain('async function getData(id)')
      expect(result.code).not.toContain('async function getData([], id)')
      expect(result.code).toContain('var $$RSC_SERVER_CACHE_0_getData = function()')
      expect(result.code).toContain('var getData = async function(...args)')
      expect(result.code).not.toContain('"use cache"')
    })

    it('leaves code unchanged when no use cache directive found', () => {
      const src = 'const x = 1;\nexport function hello() { return 42; }'
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toBe(src)
      expect(result.needsReactCache).toBe(false)
      expect(result.needsCacheWrapper).toBe(false)
      expect(result.needsRegisterRef).toBe(false)
    })

    it('generates unique index for multiple use cache functions', () => {
      const src = `
async function getData(id) {
  "use cache";
  return await db.query(id);
}
async function fetchUser(name) {
  "use cache";
  return await db.query(name);
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('$$RSC_SERVER_CACHE_0_getData')
      expect(result.code).not.toContain('$$RSC_SERVER_CACHE_0_fetchUser')
      expect(result.code).toContain('$$RSC_SERVER_CACHE_1_fetchUser')
    })

    it('does not transform non-async functions with use cache', () => {
      const src = `
function add(a, b) {
  "use cache";
  return a + b;
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toContain('$$reactCache__')
      expect(result.code).not.toContain('$$cache__')
    })

    it('strips directive from inner function body but preserves other strings', () => {
      const src = `
async function getData(id) {
  "use cache";
  "some other string";
  return await db.query(id);
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toContain('"use cache"')
      expect(result.code).toContain('"some other string"')
    })

    it('preserves non-directive expression statements after use cache', () => {
      const src = `
async function getData(id) {
  "use cache";
  1;
  return id;
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toContain('"use cache"')
      expect(result.code).toContain('1;')
    })

    it('strips both use cache and use server directives from inner body', () => {
      const src = `
async function getData() {
  "use cache";
  "use server";
  return await db.query();
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toContain('"use cache"')
      expect(result.code).not.toContain('"use server"')
    })

    it('strips extended use cache directive from inner function body', () => {
      const src = `
async function getData() {
  "use cache: stale-while-revalidate";
  return 42;
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('$$cache__("stale-while-revalidate"')
      expect(result.code).not.toContain('"use cache: stale-while-revalidate"')
    })

    it('preserves function parameter count metadata in cache wrapper', () => {
      const src = `
async function fn(a, b, c) {
  "use cache";
  return a + b + c;
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('$$cache__("default"')
      expect(result.code).toContain(
        ', 3, $$RSC_SERVER_CACHE_0_fn_INNER, Array.prototype.slice.call(arguments)',
      )
      expect(result.code).not.toContain('Array.prototype.slice.call(arguments, 0, 3)')
    })

    it('passes user parameter count (excluding bound closure slot) to cache wrapper', () => {
      const src = `
const prefix = 'test_';
async function getData(id) {
  "use cache";
  return await db.query(prefix + id);
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain(', 1, $$RSC_SERVER_CACHE_0_getData_INNER,')
    })

    it('passes all actual call arguments through the cache wrapper', () => {
      const src = `
async function fn(a, ...rest) {
  "use cache";
  return rest.length;
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('Array.prototype.slice.call(arguments)')
      expect(result.code).not.toContain('Array.prototype.slice.call(arguments, 0, 1)')
    })

    it('handles default and destructured function parameters', () => {
      const src = `
async function fn(id = 1, { slug }, [...items]) {
  "use cache";
  return id + slug + items.length;
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('async function fn(id = 1, { slug }, [...items])')
    })

    it('handles local destructuring rest and defaults', () => {
      const src = `
async function fn(input) {
  "use cache";
  const { id = 1 } = input;
  const [...items] = input.items;
  return id + items.length;
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('const { id = 1 } = input')
      expect(result.code).toContain('const [...items] = input.items')
    })

    it('generates different ref IDs for different exports', () => {
      const srcA = `
async function getData(id) {
  "use cache";
  return id;
}
`
      const srcB = `
async function fetchData(id) {
  "use cache";
  return id;
}
`
      const resultA = addon.transformUseCache(srcA, defaultOpts)
      const resultB = addon.transformUseCache(srcB, { ...defaultOpts, filename: 'test.js' })

      const idA = /registerServerReference\(\$\$RSC_SERVER_CACHE_0_getData, "([^"]+)"/.exec(
        resultA.code,
      )?.[1]
      const idB = /registerServerReference\(\$\$RSC_SERVER_CACHE_0_fetchData, "([^"]+)"/.exec(
        resultB.code,
      )?.[1]

      expect(idA).toBeTruthy()
      expect(idB).toBeTruthy()
      expect(idA).not.toBe(idB)
    })

    it('handles function with closure variables', () => {
      const src = `
const prefix = 'test_';
async function getData(id) {
  "use cache";
  return await db.query(prefix + id);
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toMatch(/\(\[\s*"[\da-f]{66}",\s*prefix\s*\]\)/)
      expect(result.code).toContain('var getData = ((')
      expect(result.code).toContain('$$ACTION_BOUND_ARGS)=>async (...args)=>')
      expect(result.code).not.toContain('async function getData([$$ACTION_ARG_0], id)')
      expect(result.code).toMatch(/"[\da-f]{66}"/)
    })

    it('transforms file-level use cache for all async exports', () => {
      const src = `
"use cache";

export async function getData(id) {
  return id;
}

export async function getOther() {
  return 42;
}

function syncHelper() {
  return 1;
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('$$cache__("default"')
      expect(result.code).not.toContain('"use cache"')
      expect(result.code).toMatch(/getData[\s\S]*\$\$cache__\("default"/)
      expect(result.code).toMatch(/getOther[\s\S]*\$\$cache__\("default"/)
      expect(result.code).not.toMatch(/syncHelper[\s\S]{0,200}\$\$cache__/)
    })

    it('does not capture body-level bindings that shadow module bindings', () => {
      const src = `
const value = 'outer';
async function getData() {
  "use cache";
  const value = 'inner';
  return value;
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('var getData = async function(...args)')
    })

    it('does not capture the transformed function name as a closure variable', () => {
      const src = `
async function getData(depth) {
  "use cache";
  return depth > 0 ? getData(depth - 1) : 0;
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('var getData = async function(...args)')
    })

    it('does not capture local function declarations that shadow module bindings', () => {
      const src = `
function helper() {}
async function getData() {
  "use cache";
  function helper() {
    return 1;
  }
  return helper();
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('function helper()')
      expect(result.code).toContain('var getData = async function(...args)')
    })

    it('does not capture local class declarations that shadow module bindings', () => {
      const src = `
const Model = null;
async function getData() {
  "use cache";
  class Model {}
  return new Model();
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('class Model')
      expect(result.code).toContain('var getData = async function(...args)')
    })

    it('preserves named default export binding', () => {
      const src = `
export default async function getData(id) {
  "use cache";
  return await db.query(id);
}

export const getDataRef = getData;
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('var getData = async function(...args)')
      expect(result.code).toContain('export default getData')
      expect(result.code).toContain('export const getDataRef = getData')
      expect(result.code).not.toContain('"use cache"')
    })

    it('transforms anonymous default export', () => {
      const src = `
export default async function(id) {
  "use cache";
  return id;
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toBe(src)
      expect(result.needsCacheWrapper).toBe(true)
      expect(result.needsRegisterRef).toBe(true)
      expect(result.code).not.toContain('"use cache"')
      expect(result.code).toContain('$$RSC_SERVER_CACHE_0_default_INNER')
      expect(result.code).toContain(
        'var $$RSC_SERVER_CACHE_DEFAULT_EXPORT = async function(...args)',
      )
      expect(result.code).toContain('export default $$RSC_SERVER_CACHE_DEFAULT_EXPORT')
    })

    it('stops directive scanning at non-string statements', () => {
      const src = `
async function getData() {
  1;
  "use cache";
  return 42;
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toBe(src)
      expect(result.needsCacheWrapper).toBe(false)
    })

    it('allows empty statements before cache directives', () => {
      const src = `
async function getData() {
  ;
  "use cache";
  return 42;
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toBe(src)
      expect(result.code).toContain('$$cache__')
    })

    it('captures module imports and destructured module bindings', () => {
      const src = `
import React, { cache as reactCache } from 'react';
import * as model from './model';

const { token, nested: alias, ...others } = config;
const [first, second] = list;

async function getData({ id, nested: { slug }, ...props }, [head], ...tail) {
  "use cache";
  return React.createElement(model.Card, {
    value: reactCache(token + alias + first + second + others.x + id + slug + props.y + head + tail.length)
  });
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toMatch(/\(\[\s*"[\da-f]{66}",/)
      for (const name of [
        'React',
        'reactCache',
        'model',
        'token',
        'alias',
        'others',
        'first',
        'second',
      ]) {
        expect(result.code).toContain(name)
      }
      expect(result.code).toContain('$$ACTION_BOUND_ARGS')
    })

    it('does not capture bindings introduced by nested scopes and patterns', () => {
      const src = `
const value = 'outer';
const item = 'outer';
const err = 'outer';
const key = 'outer';
const entry = 'outer';

async function getData(input) {
  "use cache";
  function nested({ value: localValue }) {
    return localValue;
  }
  const arrow = ({ item: localItem, ...rest }) => localItem + rest.extra;
  try {
    throw input;
  } catch ({ err }) {
    for (let key = 0; key < 1; key++) {
      key;
    }
    for (const entry in input) {
      entry;
    }
    for (const [item] of input.items) {
      item;
    }
    return nested(input) + arrow(input) + err;
  }
}
`
      const result = addon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('var getData = async function(...args)')
    })
  })
})

// ── Vite plugin integration tests (skipped in CI without Rust build) ──

const pluginDescribe = useCacheAddon ? describe : describe.skip

pluginDescribe('use-cache Vite plugin integration', () => {
  let mainPlugin: Plugin

  type TransformHook = (
    this: Readonly<{ readonly environment: { readonly name: string } }>,
    code: string,
    id: string,
  ) => Promise<{ code?: string } | null | undefined> | { code?: string } | null | undefined

  function getTransform(plugin: Plugin): TransformHook {
    const hook = plugin.transform
    if (typeof hook === 'function') {
      return async (code, id) =>
        castMock(await hook.call(castMock({ environment: { name: 'ssr' } }), code, id))
    }
    if (hook && typeof hook === 'object' && typeof hook.handler === 'function') {
      return async (code, id) =>
        castMock(await hook.handler.call(castMock({ environment: { name: 'ssr' } }), code, id))
    }
    throw new Error('expected transform hook on use-cache plugin')
  }

  beforeAll(() => {
    cleanFixtures()
    const plugins = rari({ projectRoot: process.cwd(), experimental: { useCache: true } })
    mainPlugin = plugins[0]
  })

  afterAll(() => {
    cleanFixtures()
  })

  it('transforms use cache and returns code with imports', async () => {
    const source = `
async function getData(id) {
  "use cache";
  return await db.query(id);
}
`
    const filePath = fixturePath('basic.tsx')
    writeFixture(path.basename(filePath), source)
    const result = await getTransform(mainPlugin).call(
      { environment: { name: 'rsc' } },
      source,
      filePath,
    )

    expect(result).not.toBeNull()
    expect(result).not.toContain("import { cache as $$reactCache__ } from 'react'")
    expect(result).toContain("import { $$cache__ } from '@rari/use-cache/runtime/cache-wrapper'")
    expect(result).toContain(
      "import { registerServerReference } from 'react-server-dom-rari/server'",
    )
    expect(result).not.toContain('$$reactCache__')
    expect(result).toContain('$$cache__')
    expect(result).toContain('registerServerReference')
    expect(result).not.toContain('"use cache"')
  })

  it('falls back when no use cache directive present', async () => {
    const source = 'const x = 1;\nexport function hello() { return 42; }'
    const filePath = fixturePath('no-cache.tsx')
    writeFixture(path.basename(filePath), source)
    const result = await getTransform(mainPlugin).call(
      { environment: { name: 'rsc' } },
      source,
      filePath,
    )

    // isServerComponent → true → transformServerModule returns code unchanged
    expect(result).not.toBeNull()
    expect(result).not.toContain('$$reactCache__')
    expect(result).not.toContain('$$cache__')
    expect(result).toContain('const x = 1')
  })

  it('applies use cache transform for files outside src/ (fallback path)', async () => {
    const source = `
async function getData() {
  "use cache";
  return 42;
}
`
    const dir = path.join(process.cwd(), 'tmp-test-use-cache')
    fs.mkdirSync(dir, { recursive: true })
    const filePath = path.join(dir, 'test.tsx')
    fs.writeFileSync(filePath, source, 'utf-8')

    let result
    try {
      result = await getTransform(mainPlugin).call(
        { environment: { name: 'rsc' } },
        source,
        filePath,
      )
      expect(result).not.toBeNull()
      expect(result).not.toContain('$$reactCache__')
    } finally {
      fs.rmSync(dir, { recursive: true, force: true })
    }
  })

  it('handles file with use cache followed by use server at module level', async () => {
    const source = `
"use server";

async function fetchData(id) {
  "use cache";
  return await db.query(id);
}

export async function action() {
  return "hello";
}
`
    const filePath = fixturePath('mixed.tsx')
    writeFixture(path.basename(filePath), source)
    const result = await getTransform(mainPlugin).call(
      { environment: { name: 'rsc' } },
      source,
      filePath,
    )

    expect(result).not.toBeNull()
    expect(result).not.toContain('$$reactCache__')
    expect(result).toContain('$$cache__')
    expect(result).toContain('registerServerReference')
  })

  it('processes file with multiple use cache functions', async () => {
    const source = `
async function getData(id) {
  "use cache";
  return await db.query(id);
}
async function fetchUser(name) {
  "use cache";
  return await db.query(name);
}
`
    const filePath = fixturePath('multiple.tsx')
    writeFixture(path.basename(filePath), source)
    const result = await getTransform(mainPlugin).call(
      { environment: { name: 'rsc' } },
      source,
      filePath,
    )

    expect(result).not.toBeNull()
    expect(result).toContain('$$RSC_SERVER_CACHE_0_getData_INNER')
    expect(result).toContain('$$RSC_SERVER_CACHE_1_fetchUser_INNER')
  })

  it('skips use cache transform for client environment', async () => {
    const source = `
"use client";

async function getData() {
  "use cache";
  return 42;
}

export default getData;
`
    const filePath = fixturePath('client-file.tsx')
    writeFixture(path.basename(filePath), source)

    const result = await getTransform(mainPlugin).call(
      { environment: { name: 'client' } },
      source,
      filePath,
    )

    expect(result).not.toBeNull()
    // Client modules are not transformed -- use-cache is a server-side feature
    expect(result).not.toContain('$$reactCache__')
    expect(result).not.toContain('"use client"')
    expect(result).toContain('export default getData')
  })

  it('non-TSX/JS files are skipped', async () => {
    const source = `
async function getData() {
  "use cache";
  return 42;
}
`
    const result = await getTransform(mainPlugin).call(
      { environment: { name: 'rsc' } },
      source,
      '/test/plain.css',
    )

    expect(result).toBeNull()
  })

  it('transforms use cache: remote with default cache kinds', async () => {
    const source = `
async function getData(id) {
  "use cache: remote";
  return await db.query(id);
}
`
    const filePath = fixturePath('remote.tsx')
    writeFixture(path.basename(filePath), source)
    const result = await getTransform(mainPlugin).call(
      { environment: { name: 'rsc' } },
      source,
      filePath,
    )

    expect(result).not.toBeNull()
    expect(result).toContain('$$cache__')
    expect(result).toContain('"remote"')
    expect(result).not.toContain('"use cache: remote"')
  })

  it('transforms use cache: remote with single-quoted directive', async () => {
    const source = `
async function getData() {
  'use cache: remote';
  return 42;
}
`
    const filePath = fixturePath('remote-single-quote.tsx')
    writeFixture(path.basename(filePath), source)
    const result = await getTransform(mainPlugin).call(
      { environment: { name: 'rsc' } },
      source,
      filePath,
    )

    expect(result).not.toBeNull()
    expect(result).toContain('"remote"')
  })

  it('transforms bare use cache as default kind alongside use cache: remote in same module', async () => {
    const source = `
async function bareCached() {
  "use cache";
  return 1;
}
async function remoteCached() {
  "use cache: remote";
  return 2;
}
`
    const filePath = fixturePath('mixed-default-and-remote.tsx')
    writeFixture(path.basename(filePath), source)
    const result = await getTransform(mainPlugin).call(
      { environment: { name: 'rsc' } },
      source,
      filePath,
    )

    expect(result).not.toBeNull()
    expect(result).toContain('$$cache__("default"')
    expect(result).toContain('$$cache__("remote"')
  })
})
