import { Buffer } from 'node:buffer'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { describe, expect, it, vi } from 'vite-plus/test'
import {
  buildFontFamilyStack,
  fontFormatFromPath,
  fontMimeType,
  generateFontFaceCss,
  normalizeDisplay,
  preloadLinksForFaces,
} from '../../../packages/rari/src/vite/font/css'
import {
  assertGoogleFontAssetUrl,
  buildGoogleCssUrl,
  filterFacesBySubsets,
  fontPreloadMarker,
  googleExportNameToFamily,
  parseCssFontFaces,
} from '../../../packages/rari/src/vite/font/google-loader'
import { warnGoogleFontOptions } from '../../../packages/rari/src/vite/font/google-metadata'
import { contentHash, hashedFontFileName } from '../../../packages/rari/src/vite/font/hash'
import {
  categoryFallback,
  computeFallbackOverrides,
} from '../../../packages/rari/src/vite/font/metrics'
import { extractObjectLiteral } from '../../../packages/rari/src/vite/font/parse-options'
import {
  createFontRolldownPlugin,
  transformFontSource,
} from '../../../packages/rari/src/vite/font/plugin'

describe('font parse-options', () => {
  it('parses a static options object', () => {
    const source = `localFont({ src: './Geist.woff2', variable: '--font-geist', display: 'swap' })`
    const open = source.indexOf('{')
    const parsed = extractObjectLiteral(source, open)
    expect(parsed).not.toBeNull()
    expect(parsed?.value).toEqual({
      src: './Geist.woff2',
      variable: '--font-geist',
      display: 'swap',
    })
  })

  it('parses nested src arrays', () => {
    const source = `localFont({ src: [{ path: './a.woff2', weight: 400 }, { path: './b.woff2', weight: '700' }] })`
    const parsed = extractObjectLiteral(source, source.indexOf('{'))
    expect(parsed?.value.src).toEqual([
      { path: './a.woff2', weight: 400 },
      { path: './b.woff2', weight: '700' },
    ])
  })
})

describe('font css + metrics helpers', () => {
  it('builds a font family stack with metric fallback', () => {
    expect(buildFontFamilyStack('Inter', ['system-ui'], true)).toBe(
      '"Inter", "Inter Fallback", system-ui, sans-serif',
    )
  })

  it('defaults display to swap', () => {
    expect(normalizeDisplay(undefined)).toBe('swap')
  })

  it('computes Capsize-style overrides', () => {
    const overrides = computeFallbackOverrides(
      {
        ascent: 968,
        descent: -212,
        lineGap: 0,
        unitsPerEm: 1000,
        xWidthAvg: 484,
        category: 'sans-serif',
      },
      'Arial',
    )
    expect(overrides).not.toBeNull()
    expect(overrides?.fallbackFont).toBe('Arial')
    expect(overrides?.sizeAdjust).toMatch(/%$/)
    expect(overrides?.ascentOverride).toMatch(/%$/)
  })

  it('picks serif fallback from category', () => {
    expect(categoryFallback('serif')).toBe('Times New Roman')
    expect(categoryFallback('sans-serif')).toBe('Arial')
  })
})

describe('google font loader helpers', () => {
  it('maps export names to family names', () => {
    expect(googleExportNameToFamily('Open_Sans')).toBe('Open Sans')
    expect(googleExportNameToFamily('Source_Sans_3')).toBe('Source Sans 3')
  })

  it('builds css2 URLs for weights and italics', () => {
    expect(buildGoogleCssUrl('Inter', { weight: 400 })).toBe(
      'https://fonts.googleapis.com/css2?family=Inter:wght@400&display=swap',
    )
    expect(buildGoogleCssUrl('Inter', { weight: [400, 700], style: ['normal', 'italic'] })).toBe(
      'https://fonts.googleapis.com/css2?family=Inter:ital,wght@0,400;0,700;1,400;1,700&display=swap',
    )
    expect(buildGoogleCssUrl('Inter', { weight: 'variable', style: 'italic' })).toBe(
      'https://fonts.googleapis.com/css2?family=Inter:ital,wght@1,100..900&display=swap',
    )
    expect(buildGoogleCssUrl('Inter', { weight: '200 700' })).toBe(
      'https://fonts.googleapis.com/css2?family=Inter:wght@200..700&display=swap',
    )
  })

  it('includes extra axes for variable fonts', () => {
    expect(buildGoogleCssUrl('Inter', { axes: ['opsz'] })).toBe(
      'https://fonts.googleapis.com/css2?family=Inter:opsz,wght@14..32,100..900&display=swap',
    )
    expect(buildGoogleCssUrl('Inter', { weight: '100 500', axes: ['opsz'] })).toBe(
      'https://fonts.googleapis.com/css2?family=Inter:opsz,wght@14..32,100..500&display=swap',
    )
    expect(buildGoogleCssUrl('Inter', { weight: 400, axes: ['opsz'] })).toBe(
      'https://fonts.googleapis.com/css2?family=Inter:opsz,wght@14..32,400&display=swap',
    )
    expect(buildGoogleCssUrl('Inter', { weight: [400, 700], axes: ['opsz'] })).toBe(
      'https://fonts.googleapis.com/css2?family=Inter:opsz,wght@14..32,400;14..32,700&display=swap',
    )
  })

  it('parses and filters Google CSS faces by subset comments', () => {
    const css = `
/* latin */
@font-face {
  font-family: 'Inter';
  font-style: normal;
  font-weight: 400;
  src: url(https://fonts.gstatic.com/s/inter/latin.woff2);
  unicode-range: U+0000-00FF, U+0131, U+0152-0153;
}
/* cyrillic */
@font-face {
  font-family: 'Inter';
  font-style: normal;
  font-weight: 400;
  src: url(https://fonts.gstatic.com/s/inter/cyrillic.woff2);
  unicode-range: U+0301, U+0400-045F, U+0490-0491;
}
`
    const parsed = parseCssFontFaces(css)
    expect(parsed).toHaveLength(2)
    expect(parsed[0]?.unicodeRange).toBe('U+0000-00FF, U+0131, U+0152-0153')
    expect(parsed[1]?.unicodeRange).toBe('U+0301, U+0400-045F, U+0490-0491')
    expect(filterFacesBySubsets(parsed, ['latin']).map(face => face.subset)).toEqual(['latin'])
    expect(filterFacesBySubsets(parsed, ['greek'])).toEqual([])
  })

  it('emits unicode-range in generated @font-face CSS', () => {
    const css = generateFontFaceCss(
      {
        family: 'Inter',
        style: 'normal',
        weight: '400',
        display: 'swap',
        src: [{ url: '/assets/inter.woff2', format: 'woff2' }],
        preload: true,
        unicodeRange: 'U+0000-00FF, U+0131',
        filePaths: [],
      },
      null,
      'rari_font_abc',
      null,
    )
    expect(css).toContain('unicode-range: U+0000-00FF, U+0131;')
  })

  it('warns when preload is explicitly enabled without subsets', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    warnGoogleFontOptions('Inter', {})
    expect(warn).not.toHaveBeenCalled()

    warn.mockClear()
    warnGoogleFontOptions('Inter', { preload: true })
    expect(warn).toHaveBeenCalledOnce()
    expect(warn.mock.calls[0]?.[0]).toContain('subsets')

    warn.mockClear()
    warnGoogleFontOptions('Inter', { subsets: ['latin'] })
    expect(warn).not.toHaveBeenCalled()

    warn.mockClear()
    warnGoogleFontOptions('Inter', { preload: false })
    expect(warn).not.toHaveBeenCalled()
    warn.mockRestore()
  })

  it('rejects non-allowlisted Google font asset URLs', () => {
    expect(assertGoogleFontAssetUrl('https://fonts.gstatic.com/s/inter.woff2').hostname).toBe(
      'fonts.gstatic.com',
    )
    expect(() => assertGoogleFontAssetUrl('http://fonts.gstatic.com/s/inter.woff2')).toThrow(
      /HTTPS/,
    )
    expect(() => assertGoogleFontAssetUrl('https://evil.example/font.woff2')).toThrow(/allowlisted/)
  })

  it('warns on unknown subsets for a known Google family', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    warnGoogleFontOptions('Inter', { subsets: ['latin', 'not-a-subset'], preload: false })
    expect(warn).toHaveBeenCalledOnce()
    expect(warn.mock.calls[0]?.[0]).toContain('unknown subset')
    expect(warn.mock.calls[0]?.[0]).toContain('not-a-subset')
    warn.mockRestore()
  })
})

describe('font hashing', () => {
  it('hashes content and builds asset names', () => {
    const hash = contentHash(Buffer.from('abc'))
    expect(hash).toHaveLength(8)
    expect(hashedFontFileName('/tmp/Geist.woff2', hash, 'assets')).toBe(
      `assets/Geist-${hash}.woff2`,
    )
  })
})

describe('transformFontSource (local)', () => {
  it('rewrites localFont calls into static objects and emits assets', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-font-'))
    const fontPath = path.join(dir, 'Geist.woff2')
    fs.writeFileSync(fontPath, Buffer.from('fake-font-bytes'))

    const importer = path.join(dir, 'app.tsx')
    const code = `
import localFont from 'rari/font/local'

const geist = localFont({
  src: './Geist.woff2',
  variable: '--font-geist',
  adjustFontFallback: false,
})

export default function Page() {
  return <html className={geist.variable} />
}
`
    fs.writeFileSync(importer, code)

    const result = await transformFontSource(code, importer, dir, 'assets')
    expect(result).not.toBeNull()
    expect(result?.code).not.toContain("from 'rari/font/local'")
    expect(result?.code).toContain('className:')
    expect(result?.code).toContain('variable:')
    expect(result?.code).toMatch(/import ["']\\u0000rari-font-css:/)
    expect(result?.assets.length).toBe(1)
    expect(result?.assets[0]?.fileName).toMatch(/^assets\/Geist-[a-f0-9]{8}\.woff2$/)
    expect(result?.cssModules[0]?.css).toContain('@font-face')
    expect(result?.cssModules[0]?.css).toContain('--font-geist')
    expect(result?.preloadUrls).toHaveLength(1)
    expect(result?.preloadUrls[0]).toMatch(/^\/assets\/Geist-[a-f0-9]{8}\.woff2$/)
  })

  it('emits custom declarations into @font-face CSS', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-font-'))
    fs.writeFileSync(path.join(dir, 'Geist.woff2'), Buffer.from('decls'))
    const importer = path.join(dir, 'app.tsx')
    const code = `import localFont from 'rari/font/local'
const geist = localFont({
  src: './Geist.woff2',
  adjustFontFallback: false,
  declarations: [{ prop: 'ascent-override', value: '90%' }],
})
`
    const result = await transformFontSource(code, importer, dir, 'assets')
    expect(result?.cssModules[0]?.css).toContain('ascent-override: 90%;')
  })

  it('keeps use client above generated CSS imports', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-font-'))
    fs.writeFileSync(path.join(dir, 'Geist.woff2'), Buffer.from('client-font'))
    const importer = path.join(dir, 'button.tsx')
    const code = `'use client'

import localFont from 'rari/font/local'

const geist = localFont({ src: './Geist.woff2', adjustFontFallback: false })
`
    const result = await transformFontSource(code, importer, dir, 'assets')
    expect(result?.code).toMatch(/^'use client'\s*\nimport "/)
    expect(result?.code).toContain('rari-font-css:')
  })

  it('rejects namespace font imports even when a valid import is present', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-font-'))
    const importer = path.join(dir, 'app.tsx')
    const code = `
import { Inter } from 'rari/font/google'
import * as localFonts from 'rari/font/local'
`
    await expect(transformFontSource(code, importer, dir, 'assets')).rejects.toThrow(
      /Namespace and side-effect imports/,
    )
  })

  it('ignores font binding names inside comments and strings', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-font-'))
    fs.writeFileSync(path.join(dir, 'Geist.woff2'), Buffer.from('comment-font'))
    const importer = path.join(dir, 'app.tsx')
    const code = `import localFont from 'rari/font/local'
// localFont({ src: './missing.woff2' })
const note = "localFont({ src: './missing.woff2' })"
const geist = localFont({ src: './Geist.woff2', adjustFontFallback: false })
`
    const result = await transformFontSource(code, importer, dir, 'assets')
    expect(result?.code).toContain("// localFont({ src: './missing.woff2' })")
    expect(result?.code).toContain('"localFont({ src: \'./missing.woff2\' })"')
    expect(result?.code).toContain('className:')
    expect(result?.assets).toHaveLength(1)
  })

  it('transforms every matching local font import statement', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-font-'))
    const importer = path.join(dir, 'app.tsx')
    fs.writeFileSync(path.join(dir, 'a.woff2'), Buffer.from('a'))
    fs.writeFileSync(path.join(dir, 'b.woff2'), Buffer.from('b'))
    const code = `
import localFont from 'rari/font/local'
import anotherFont from 'rari/font/local'
const a = localFont({ src: './a.woff2', adjustFontFallback: false })
const b = anotherFont({ src: './b.woff2', adjustFontFallback: false })
`
    const result = await transformFontSource(code, importer, dir, 'assets')
    expect(result?.code).not.toContain("from 'rari/font/local'")
    expect(result?.assets).toHaveLength(2)
  })

  it('maps otf files to opentype format and font/otf preload type', () => {
    expect(fontFormatFromPath('/fonts/Display.otf')).toBe('opentype')
    expect(fontMimeType('opentype')).toBe('font/otf')
    expect(
      preloadLinksForFaces([
        {
          family: 'Display',
          style: 'normal',
          weight: '400',
          display: 'swap',
          src: [{ url: '/assets/Display.otf', format: 'opentype' }],
          preload: true,
          filePaths: [],
        },
      ]),
    ).toContain('type="font/otf"')
  })

  it('writes assets through the Rolldown plugin', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-font-'))
    const outDir = path.join(dir, 'dist')
    const fontPath = path.join(dir, 'Geist.woff2')
    fs.writeFileSync(fontPath, Buffer.from('rolldown-font'))
    const importer = path.join(dir, 'layout.tsx')
    const code = `import localFont from 'rari/font/local'
const geist = localFont({ src: './Geist.woff2', adjustFontFallback: false })
`
    fs.writeFileSync(importer, code)

    const cssCollector: string[] = []
    const preloadCollector: string[] = []
    const plugin = createFontRolldownPlugin(dir, 'assets', outDir, cssCollector, preloadCollector)
    const transformed = await plugin.transform(code, importer)
    expect(transformed?.code).toContain('className:')

    const cssImport = /import ["'](\\u0000rari-font-css:[^"']+)["']/.exec(transformed?.code ?? '')
    expect(cssImport).not.toBeNull()
    expect(cssImport![1]).toEqual(expect.any(String))
    const cssId = cssImport![1].replace('\\u0000', '\0')

    expect(plugin.resolveId(cssId)).toBe(cssId)
    expect(plugin.load(cssId)?.code).toBe('export {}')
    expect(cssCollector).toHaveLength(1)
    expect(cssCollector[0]).toContain('@font-face')
    expect(preloadCollector).toHaveLength(1)
    expect(preloadCollector[0]).toMatch(/^preload:\/assets\/Geist-[a-f0-9]{8}\.woff2$/)
    expect(fontPreloadMarker('/assets/x.woff2')).toBe('preload:/assets/x.woff2')

    const files = fs.readdirSync(path.join(outDir, 'assets'))
    expect(files.some(name => name.startsWith('Geist-') && name.endsWith('.woff2'))).toBe(true)
  })
})
