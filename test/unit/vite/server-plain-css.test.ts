import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { isRecord } from '@rari/shared/utils/type-guards'
import { ServerComponentBuilder } from '@rari/vite/server/build'
import { afterEach, describe, expect, it } from 'vite-plus/test'

function readManifestCssHrefs(manifestPath: string): string[] {
  const parsed: unknown = JSON.parse(fs.readFileSync(manifestPath, 'utf-8'))
  if (!isRecord(parsed) || !isRecord(parsed.components)) {
    throw new Error('manifest.json missing components')
  }

  const entries = Object.values(parsed.components)
  const first = entries[0]
  if (!isRecord(first) || !Array.isArray(first.css)) {
    throw new Error('manifest component missing css array')
  }

  return first.css.filter((href): href is string => typeof href === 'string')
}

describe('server plain css imports', () => {
  let dir = ''

  afterEach(() => {
    if (dir !== '') fs.rmSync(dir, { recursive: true, force: true })
  })

  it('records page plain css in the component css asset', async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-plain-css-'))
    const appDir = path.join(dir, 'src', 'app')
    fs.mkdirSync(appDir, { recursive: true })
    fs.writeFileSync(path.join(appDir, 'page.css'), '.page { color: red; }')
    const pagePath = path.join(appDir, 'page.tsx')
    fs.writeFileSync(
      pagePath,
      `import './page.css'\nexport default function Page() { return null }\n`,
    )

    const outDir = path.join(dir, 'dist')
    fs.mkdirSync(path.join(outDir, 'server'), { recursive: true })
    fs.writeFileSync(
      path.join(outDir, 'server', 'manifest.json'),
      JSON.stringify({ components: {}, buildTime: new Date().toISOString() }),
    )

    const builder = new ServerComponentBuilder(dir, {
      outDir: 'dist',
      rscDir: 'server',
      manifestPath: 'server/manifest.json',
      minify: false,
      alias: {},
    })

    const result = await builder.rebuildComponent(pagePath)
    expect(result.success).toBe(true)

    const cssHrefs = readManifestCssHrefs(path.join(outDir, 'server', 'manifest.json'))
    expect(cssHrefs.length).toBeGreaterThan(0)

    const cssHref = cssHrefs[0]
    expect(cssHref).toBeTypeOf('string')
    const cssPath = path.join(outDir, cssHref.replace(/^\//, ''))
    expect(fs.readFileSync(cssPath, 'utf-8')).toContain('.page { color: red; }')
  })

  it('records bare package css imports in the component css asset', async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-pkg-css-'))
    const pkgDir = path.join(dir, 'node_modules', 'acme-ui')
    fs.mkdirSync(pkgDir, { recursive: true })
    fs.writeFileSync(path.join(pkgDir, 'package.json'), JSON.stringify({ name: 'acme-ui' }))
    fs.writeFileSync(path.join(pkgDir, 'styles.css'), '.acme { color: blue; }')

    const appDir = path.join(dir, 'src', 'app')
    fs.mkdirSync(appDir, { recursive: true })
    const pagePath = path.join(appDir, 'page.tsx')
    fs.writeFileSync(
      pagePath,
      `import 'acme-ui/styles.css'\nexport default function Page() { return null }\n`,
    )

    const outDir = path.join(dir, 'dist')
    fs.mkdirSync(path.join(outDir, 'server'), { recursive: true })
    fs.writeFileSync(
      path.join(outDir, 'server', 'manifest.json'),
      JSON.stringify({ components: {}, buildTime: new Date().toISOString() }),
    )
    fs.writeFileSync(path.join(dir, 'package.json'), JSON.stringify({ name: 'app' }))

    const builder = new ServerComponentBuilder(dir, {
      outDir: 'dist',
      rscDir: 'server',
      manifestPath: 'server/manifest.json',
      minify: false,
      alias: {},
    })

    const result = await builder.rebuildComponent(pagePath)
    expect(result.success).toBe(true)

    const cssHrefs = readManifestCssHrefs(path.join(outDir, 'server', 'manifest.json'))
    expect(cssHrefs.length).toBeGreaterThan(0)

    const cssHref = cssHrefs[0]
    expect(cssHref).toBeTypeOf('string')
    const cssPath = path.join(outDir, cssHref.replace(/^\//, ''))
    expect(fs.readFileSync(cssPath, 'utf-8')).toContain('.acme { color: blue; }')
  })

  it('preserves default exports for css?raw and css?url without stylesheet assets', async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-css-query-'))
    const appDir = path.join(dir, 'src', 'app')
    fs.mkdirSync(appDir, { recursive: true })
    fs.writeFileSync(path.join(appDir, 'theme.css'), '.theme { color: green; }')
    const pagePath = path.join(appDir, 'page.tsx')
    fs.writeFileSync(
      pagePath,
      `import raw from './theme.css?raw'\nimport href from './theme.css?url'\nexport default function Page() { return raw + href }\n`,
    )

    const outDir = path.join(dir, 'dist')
    fs.mkdirSync(path.join(outDir, 'server'), { recursive: true })
    fs.writeFileSync(
      path.join(outDir, 'server', 'manifest.json'),
      JSON.stringify({ components: {}, buildTime: new Date().toISOString() }),
    )

    const builder = new ServerComponentBuilder(dir, {
      outDir: 'dist',
      rscDir: 'server',
      manifestPath: 'server/manifest.json',
      minify: false,
      alias: {},
    })

    const result = await builder.rebuildComponent(pagePath)
    expect(result.success).toBe(true)

    const bundle = fs.readFileSync(result.bundlePath, 'utf-8')
    expect(bundle).toContain('.theme { color: green; }')

    const urlMatch = /\/assets\/theme-[a-z0-9]+\.css/.exec(bundle)
    expect(urlMatch, `bundle missing emitted asset URL:\n${bundle}`).not.toBeNull()
    const exportedUrl = urlMatch![0]
    expect(exportedUrl).toMatch(/^\/assets\/theme-[a-z0-9]+\.css$/)

    const emittedPath = path.join(outDir, exportedUrl.replace(/^\//, ''))
    expect(fs.existsSync(emittedPath)).toBe(true)
    expect(fs.readFileSync(emittedPath, 'utf-8')).toContain('.theme { color: green; }')

    const manifestPath = path.join(outDir, 'server', 'manifest.json')
    const parsed: unknown = JSON.parse(fs.readFileSync(manifestPath, 'utf-8'))
    if (!isRecord(parsed) || !isRecord(parsed.components)) {
      throw new Error('manifest.json missing components')
    }
    const entry = Object.values(parsed.components)[0]
    if (!isRecord(entry) || !Array.isArray(entry.css)) {
      throw new Error('manifest component missing css array')
    }
    expect(entry.css).toEqual([])
  })

  it('skips layout-owned plain css from server assets (client head owns it)', async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-layout-css-skip-'))
    const appDir = path.join(dir, 'src', 'app')
    fs.mkdirSync(appDir, { recursive: true })
    fs.writeFileSync(
      path.join(appDir, 'globals.css'),
      "@import 'tailwindcss';\n:root { --brand: red; }\n",
    )
    const layoutPath = path.join(appDir, 'layout.tsx')
    fs.writeFileSync(
      layoutPath,
      `import './globals.css'\nexport default function Layout({ children }) { return children }\n`,
    )

    const outDir = path.join(dir, 'dist')
    fs.mkdirSync(path.join(outDir, 'server'), { recursive: true })
    fs.writeFileSync(
      path.join(outDir, 'server', 'manifest.json'),
      JSON.stringify({ components: {}, buildTime: new Date().toISOString() }),
    )

    const builder = new ServerComponentBuilder(dir, {
      outDir: 'dist',
      rscDir: 'server',
      manifestPath: 'server/manifest.json',
      minify: false,
      alias: {},
    })

    const result = await builder.rebuildComponent(layoutPath)
    expect(result.success).toBe(true)

    const manifestPath = path.join(outDir, 'server', 'manifest.json')
    const parsed: unknown = JSON.parse(fs.readFileSync(manifestPath, 'utf-8'))
    if (!isRecord(parsed) || !isRecord(parsed.components)) {
      throw new Error('manifest.json missing components')
    }
    const entry = Object.values(parsed.components)[0]
    if (!isRecord(entry) || !Array.isArray(entry.css)) {
      throw new Error('manifest component missing css array')
    }
    expect(entry.css).toEqual([])
  })

  it('emits local rules after stripping bare package @imports', async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-tw-mixed-css-'))
    const appDir = path.join(dir, 'src', 'app')
    fs.mkdirSync(appDir, { recursive: true })
    fs.writeFileSync(
      path.join(appDir, 'page.css'),
      "@import 'tailwindcss';\n.page { color: red; }\n",
    )
    const pagePath = path.join(appDir, 'page.tsx')
    fs.writeFileSync(
      pagePath,
      `import './page.css'\nexport default function Page() { return null }\n`,
    )

    const outDir = path.join(dir, 'dist')
    fs.mkdirSync(path.join(outDir, 'server'), { recursive: true })
    fs.writeFileSync(
      path.join(outDir, 'server', 'manifest.json'),
      JSON.stringify({ components: {}, buildTime: new Date().toISOString() }),
    )

    const builder = new ServerComponentBuilder(dir, {
      outDir: 'dist',
      rscDir: 'server',
      manifestPath: 'server/manifest.json',
      minify: false,
      alias: {},
    })

    const result = await builder.rebuildComponent(pagePath)
    expect(result.success).toBe(true)

    const cssHrefs = readManifestCssHrefs(path.join(outDir, 'server', 'manifest.json'))
    expect(cssHrefs.length).toBeGreaterThan(0)
    const cssHref = cssHrefs[0]
    expect(cssHref).toBeTypeOf('string')
    const cssPath = path.join(outDir, cssHref.replace(/^\//, ''))
    const emitted = fs.readFileSync(cssPath, 'utf-8')
    expect(emitted).toContain('.page { color: red; }')
    expect(emitted).not.toContain('@import')
  })

  it('strips qualified bare package @imports including layer and media', async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-tw-qualified-css-'))
    const appDir = path.join(dir, 'src', 'app')
    fs.mkdirSync(appDir, { recursive: true })
    fs.writeFileSync(
      path.join(appDir, 'page.css'),
      '@import \'tailwindcss\' layer(base);\n@import "acme" layer(utilities) screen and (min-width: 40rem);\n.page { color: teal; }\n',
    )
    const pagePath = path.join(appDir, 'page.tsx')
    fs.writeFileSync(
      pagePath,
      `import './page.css'\nexport default function Page() { return null }\n`,
    )

    const outDir = path.join(dir, 'dist')
    fs.mkdirSync(path.join(outDir, 'server'), { recursive: true })
    fs.writeFileSync(
      path.join(outDir, 'server', 'manifest.json'),
      JSON.stringify({ components: {}, buildTime: new Date().toISOString() }),
    )

    const builder = new ServerComponentBuilder(dir, {
      outDir: 'dist',
      rscDir: 'server',
      manifestPath: 'server/manifest.json',
      minify: false,
      alias: {},
    })

    const result = await builder.rebuildComponent(pagePath)
    expect(result.success).toBe(true)

    const cssHrefs = readManifestCssHrefs(path.join(outDir, 'server', 'manifest.json'))
    expect(cssHrefs.length).toBeGreaterThan(0)
    const cssHref = cssHrefs[0]
    expect(cssHref).toBeTypeOf('string')
    const emitted = fs.readFileSync(path.join(outDir, cssHref.replace(/^\//, '')), 'utf-8')
    expect(emitted).toContain('.page { color: teal; }')
    expect(emitted).not.toContain('@import')
    expect(emitted).not.toContain('layer(')
    expect(emitted).not.toContain('screen and')
  })

  it('inlines nested local relative @imports into the server css asset', async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-nested-css-'))
    const appDir = path.join(dir, 'src', 'app')
    fs.mkdirSync(appDir, { recursive: true })
    fs.writeFileSync(path.join(appDir, 'nested.css'), '.nested { color: blue; }\n')
    fs.writeFileSync(
      path.join(appDir, 'page.css'),
      "@import './nested.css';\n.page { color: red; }\n",
    )
    const pagePath = path.join(appDir, 'page.tsx')
    fs.writeFileSync(
      pagePath,
      `import './page.css'\nexport default function Page() { return null }\n`,
    )

    const outDir = path.join(dir, 'dist')
    fs.mkdirSync(path.join(outDir, 'server'), { recursive: true })
    fs.writeFileSync(
      path.join(outDir, 'server', 'manifest.json'),
      JSON.stringify({ components: {}, buildTime: new Date().toISOString() }),
    )

    const builder = new ServerComponentBuilder(dir, {
      outDir: 'dist',
      rscDir: 'server',
      manifestPath: 'server/manifest.json',
      minify: false,
      alias: {},
    })

    const result = await builder.rebuildComponent(pagePath)
    expect(result.success).toBe(true)

    const nestedCssHrefs = readManifestCssHrefs(path.join(outDir, 'server', 'manifest.json'))
    expect(nestedCssHrefs.length).toBeGreaterThan(0)
    const nestedCssHref = nestedCssHrefs[0]
    expect(nestedCssHref).toBeTypeOf('string')
    const emitted = fs.readFileSync(path.join(outDir, nestedCssHref.replace(/^\//, '')), 'utf-8')
    expect(emitted).toContain('.nested { color: blue; }')
    expect(emitted).toContain('.page { color: red; }')
    expect(emitted).not.toContain("@import './nested.css'")
  })

  it('preserves layer, supports, and media qualifiers when inlining local @imports', async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-qualified-inline-css-'))
    const appDir = path.join(dir, 'src', 'app')
    fs.mkdirSync(appDir, { recursive: true })
    fs.writeFileSync(path.join(appDir, 'layer.css'), '.layered { color: navy; }\n')
    fs.writeFileSync(path.join(appDir, 'supports.css'), '.supported { display: grid; }\n')
    fs.writeFileSync(path.join(appDir, 'print.css'), '.printed { color: black; }\n')
    fs.writeFileSync(
      path.join(appDir, 'page.css'),
      [
        "@import './layer.css' layer(base);",
        "@import './supports.css' supports(display: grid);",
        "@import './print.css' print;",
        '.page { color: red; }',
        '',
      ].join('\n'),
    )
    const pagePath = path.join(appDir, 'page.tsx')
    fs.writeFileSync(
      pagePath,
      `import './page.css'\nexport default function Page() { return null }\n`,
    )

    const outDir = path.join(dir, 'dist')
    fs.mkdirSync(path.join(outDir, 'server'), { recursive: true })
    fs.writeFileSync(
      path.join(outDir, 'server', 'manifest.json'),
      JSON.stringify({ components: {}, buildTime: new Date().toISOString() }),
    )

    const builder = new ServerComponentBuilder(dir, {
      outDir: 'dist',
      rscDir: 'server',
      manifestPath: 'server/manifest.json',
      minify: false,
      alias: {},
    })

    const result = await builder.rebuildComponent(pagePath)
    expect(result.success).toBe(true)

    const cssHrefs = readManifestCssHrefs(path.join(outDir, 'server', 'manifest.json'))
    expect(cssHrefs.length).toBeGreaterThan(0)
    const cssHref = cssHrefs[0]
    expect(cssHref).toBeTypeOf('string')
    const emitted = fs.readFileSync(path.join(outDir, cssHref.replace(/^\//, '')), 'utf-8')

    expect(emitted).toContain('@layer base')
    expect(emitted).toContain('.layered { color: navy; }')
    expect(emitted).toContain('@supports (display: grid)')
    expect(emitted).toContain('.supported { display: grid; }')
    expect(emitted).toContain('@media print')
    expect(emitted).toContain('.printed { color: black; }')
    expect(emitted).toContain('.page { color: red; }')
    expect(emitted).not.toMatch(/@import\s+['"]\.\//)
  })

  it('inlines unquoted url(./...) local @imports into the component css asset', async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-unquoted-url-css-'))
    const appDir = path.join(dir, 'src', 'app')
    fs.mkdirSync(appDir, { recursive: true })
    fs.writeFileSync(path.join(appDir, 'nested.css'), '.nested { color: purple; }\n')
    fs.writeFileSync(
      path.join(appDir, 'page.css'),
      '@import url(./nested.css);\n.page { color: red; }\n',
    )
    const pagePath = path.join(appDir, 'page.tsx')
    fs.writeFileSync(
      pagePath,
      `import './page.css'\nexport default function Page() { return null }\n`,
    )

    const outDir = path.join(dir, 'dist')
    fs.mkdirSync(path.join(outDir, 'server'), { recursive: true })
    fs.writeFileSync(
      path.join(outDir, 'server', 'manifest.json'),
      JSON.stringify({ components: {}, buildTime: new Date().toISOString() }),
    )

    const builder = new ServerComponentBuilder(dir, {
      outDir: 'dist',
      rscDir: 'server',
      manifestPath: 'server/manifest.json',
      minify: false,
      alias: {},
    })

    const result = await builder.rebuildComponent(pagePath)
    expect(result.success).toBe(true)
    expect(fs.existsSync(result.bundlePath)).toBe(true)

    const cssHrefs = readManifestCssHrefs(path.join(outDir, 'server', 'manifest.json'))
    expect(cssHrefs.length).toBeGreaterThan(0)
    const cssHref = cssHrefs[0]
    expect(cssHref).toBeTypeOf('string')
    expect(cssHref.startsWith('/assets/server/')).toBe(true)
    const emitted = fs.readFileSync(path.join(outDir, cssHref.replace(/^\//, '')), 'utf-8')
    expect(emitted).toContain('.nested { color: purple; }')
    expect(emitted).toContain('.page { color: red; }')
    expect(emitted).not.toContain('url(./nested.css)')
  })

  it('inlines the same local file under different qualifiers', async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-dup-qualified-css-'))
    const appDir = path.join(dir, 'src', 'app')
    fs.mkdirSync(appDir, { recursive: true })
    fs.writeFileSync(path.join(appDir, 'theme.css'), '.theme { color: orange; }\n')
    fs.writeFileSync(
      path.join(appDir, 'page.css'),
      [
        "@import './theme.css' layer(base);",
        "@import './theme.css' layer(utilities);",
        '.page { color: red; }',
        '',
      ].join('\n'),
    )
    const pagePath = path.join(appDir, 'page.tsx')
    fs.writeFileSync(
      pagePath,
      `import './page.css'\nexport default function Page() { return null }\n`,
    )

    const outDir = path.join(dir, 'dist')
    fs.mkdirSync(path.join(outDir, 'server'), { recursive: true })
    fs.writeFileSync(
      path.join(outDir, 'server', 'manifest.json'),
      JSON.stringify({ components: {}, buildTime: new Date().toISOString() }),
    )

    const builder = new ServerComponentBuilder(dir, {
      outDir: 'dist',
      rscDir: 'server',
      manifestPath: 'server/manifest.json',
      minify: false,
      alias: {},
    })

    const result = await builder.rebuildComponent(pagePath)
    expect(result.success).toBe(true)

    const cssHrefs = readManifestCssHrefs(path.join(outDir, 'server', 'manifest.json'))
    expect(cssHrefs.length).toBeGreaterThan(0)
    const cssHref = cssHrefs[0]
    expect(cssHref).toBeTypeOf('string')
    const emitted = fs.readFileSync(path.join(outDir, cssHref.replace(/^\//, '')), 'utf-8')

    expect(emitted).toContain('@layer base')
    expect(emitted).toContain('@layer utilities')
    expect(emitted.match(/\.theme \{ color: orange; \}/g)?.length).toBe(2)
    expect(emitted).toContain('.page { color: red; }')
  })
})
