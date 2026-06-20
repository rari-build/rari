import fs from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import { rari } from '@rari/vite'
import { afterAll, beforeAll, describe, expect, it } from 'vite-plus/test'

const FIXTURE_DIR = path.join(process.cwd(), 'test/fixtures/use-cache')

let useCacheAddon = null

try {
  const repoRoot = process.cwd()
  const ext = process.platform === 'win32' ? '.dll' : process.platform === 'darwin' ? '.dylib' : '.so'

  const candidates = [
    path.join(repoRoot, 'target/release/rari_use_cache.node'),
    path.join(repoRoot, `target/release/librari_use_cache${ext}`),
    path.join(repoRoot, 'target/debug/rari_use_cache.node'),
    path.join(repoRoot, `target/debug/librari_use_cache${ext}`),
  ]

  for (const addonPath of candidates) {
    if (fs.existsSync(addonPath)) {
      const nodeRequire = createRequire(import.meta.url)
      useCacheAddon = nodeRequire(addonPath)
      break
    }
  }
}
catch {
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
  describe('detectUseCache', () => {
    it('returns true for double-quoted use cache directive', () => {
      expect(useCacheAddon.detectUseCache('"use cache";')).toBe(true)
    })

    it('returns true for single-quoted use cache directive', () => {
      expect(useCacheAddon.detectUseCache('\'use cache\';')).toBe(true)
    })

    it('returns true for inline use cache directive', () => {
      expect(useCacheAddon.detectUseCache('"use cache"')).toBe(true)
    })

    it('returns false for plain code', () => {
      expect(useCacheAddon.detectUseCache('const x = 1;')).toBe(false)
    })

    it('returns false for empty string', () => {
      expect(useCacheAddon.detectUseCache('')).toBe(false)
    })

    it('returns false for similar-looking strings', () => {
      expect(useCacheAddon.detectUseCache('"use cached"')).toBe(false)
      expect(useCacheAddon.detectUseCache('"cache"')).toBe(false)
    })

    it('detects backtick cache directives with custom kinds', () => {
      expect(useCacheAddon.detectUseCache('`use cache: stale-while-revalidate`')).toBe(true)
      expect(useCacheAddon.detectUseCache('`use cache:`')).toBe(false)
    })
  })

  describe('transformUseCache', () => {
    const defaultOpts = {
      filename: 'test.js',
      hashSalt: 'rari-use-cache-v1',
      cacheKinds: ['default'],
    }

    it('transforms async function with use cache directive', () => {
      const src = `
async function getData(id) {
  "use cache";
  return await db.query(id);
}
`
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('$$RSC_SERVER_CACHE_0_getData')
      expect(result.code).not.toContain('$$RSC_SERVER_CACHE_0_fetchUser')
      expect(result.code).toContain('$$RSC_SERVER_CACHE_1_fetchUser')
    })

    it('transforms non-async function with use cache', () => {
      const src = `
function add(a, b) {
  "use cache";
  return a + b;
}
`
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toContain('$$reactCache__')
      expect(result.code).toContain('$$cache__')
    })

    it('strips directive from inner function body but preserves other strings', () => {
      const src = `
async function getData(id) {
  "use cache";
  "some other string";
  return await db.query(id);
}
`
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('$$cache__("default"')
      expect(result.code).toContain(', 3, $$RSC_SERVER_CACHE_0_fn_INNER, Array.prototype.slice.call(arguments)')
      expect(result.code).not.toContain('Array.prototype.slice.call(arguments, 0, 3)')
    })

    it('reports inner-function arity (params + bound args) when capturing module-level variables', () => {
      const src = `
const prefix = 'test_';
async function getData(id) {
  "use cache";
  return await db.query(prefix + id);
}
`
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain(', 2, $$RSC_SERVER_CACHE_0_getData_INNER,')
    })

    it('passes all actual call arguments through the cache wrapper', () => {
      const src = `
async function fn(a, ...rest) {
  "use cache";
  return rest.length;
}
`
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('async function fn(id = 1, { slug }, [...items])')
      expect(result.code).not.toContain('encodeBoundArgs')
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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('const { id = 1 } = input')
      expect(result.code).toContain('const [...items] = input.items')
      expect(result.code).not.toContain('encodeBoundArgs')
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
      const resultA = useCacheAddon.transformUseCache(srcA, defaultOpts)
      const resultB = useCacheAddon.transformUseCache(srcB, { ...defaultOpts, filename: 'test.js' })

      const idA = resultA.code.match(/registerServerReference\(\$\$RSC_SERVER_CACHE_0_getData, "([^"]+)"/)?.[1]
      const idB = resultB.code.match(/registerServerReference\(\$\$RSC_SERVER_CACHE_0_fetchData, "([^"]+)"/)?.[1]

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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('encodeBoundArgs')
      expect(result.code).toContain('var getData = ((')
      expect(result.code).toContain('$$ACTION_BOUND_ARGS)=>async (...args)=>')
      expect(result.code).toContain('encodeBoundArgs(')
      expect(result.code).not.toContain('async function getData([$$ACTION_ARG_0], id)')
      expect(result.code).toMatch(/"[\da-f]{42}"/)
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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toContain('encodeBoundArgs')
      expect(result.code).toContain('var getData = async function(...args)')
    })

    it('does not capture the transformed function name as a closure variable', () => {
      const src = `
async function getData(depth) {
  "use cache";
  return depth > 0 ? getData(depth - 1) : 0;
}
`
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toContain('encodeBoundArgs')
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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toContain('encodeBoundArgs')
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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toContain('encodeBoundArgs')
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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toBe(src)
      expect(result.needsCacheWrapper).toBe(true)
      expect(result.needsRegisterRef).toBe(true)
      expect(result.code).not.toContain('"use cache"')
      expect(result.code).toContain('$$RSC_SERVER_CACHE_0_default_INNER')
      expect(result.code).toContain('var $$RSC_SERVER_CACHE_DEFAULT_EXPORT = async function(...args)')
      expect(result.code).toContain('export default $$RSC_SERVER_CACHE_DEFAULT_EXPORT')
    })

    it('uses default cache kind when options list is empty', () => {
      const src = `
async function getData() {
  "use cache";
  return 42;
}
`
      const result = useCacheAddon.transformUseCache(src, { ...defaultOpts, cacheKinds: [] })

      expect(result.code).toContain('$$cache__("default"')
    })

    it('stops directive scanning at non-string statements', () => {
      const src = `
async function getData() {
  1;
  "use cache";
  return 42;
}
`
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).toContain('encodeBoundArgs')
      for (const name of ['React', 'reactCache', 'model', 'token', 'alias', 'others', 'first', 'second']) {
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
      const result = useCacheAddon.transformUseCache(src, defaultOpts)

      expect(result.code).not.toContain('encodeBoundArgs')
      expect(result.code).toContain('var getData = async function(...args)')
    })
  })
})

// ── Vite plugin integration tests (skipped in CI without Rust build) ──

const pluginDescribe = useCacheAddon ? describe : describe.skip

pluginDescribe('use-cache Vite plugin integration', () => {
  let mainPlugin: any

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
    const p = mainPlugin.transform as any
    const result = await p.call(
      { environment: { name: 'rsc' } },
      source,
      filePath,
    )

    expect(result).not.toBeNull()
    expect(result).not.toContain('import { cache as $$reactCache__ } from \'react\'')
    expect(result).toContain('import { $$cache__, encodeBoundArgs } from \'@rari/use-cache/runtime/cache-wrapper\'')
    expect(result).toContain('import { registerServerReference } from \'rari/runtime/react-server-dom-shim\'')
    expect(result).not.toContain('$$reactCache__')
    expect(result).toContain('$$cache__')
    expect(result).toContain('registerServerReference')
    expect(result).not.toContain('"use cache"')
  })

  it('falls back when no use cache directive present', async () => {
    const source = 'const x = 1;\nexport function hello() { return 42; }'
    const filePath = fixturePath('no-cache.tsx')
    writeFixture(path.basename(filePath), source)
    const p = mainPlugin.transform as any
    const result = await p.call(
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

    const p = mainPlugin.transform as any
    let result
    try {
      result = await p.call(
        { environment: { name: 'rsc' } },
        source,
        filePath,
      )
      expect(result).not.toBeNull()
      expect(result).not.toContain('$$reactCache__')
    }
    finally {
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
    const p = mainPlugin.transform as any
    const result = await p.call(
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
    const p = mainPlugin.transform as any
    const result = await p.call(
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

    const p = mainPlugin.transform as any
    const result = await p.call(
      { environment: { name: 'client' } },
      source,
      filePath,
    )

    expect(result).not.toBeNull()
    // Client modules are not transformed — use-cache is a server-side feature
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
    const p = mainPlugin.transform as any
    const result = await p.call(
      { environment: { name: 'rsc' } },
      source,
      '/test/plain.css',
    )

    expect(result).toBeNull()
  })
})
