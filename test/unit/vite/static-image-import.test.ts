import { Buffer } from 'node:buffer'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { describe, expect, it } from 'vite-plus/test'
import { parseJsonRecord } from '../../../packages/rari/src/shared/utils/type-guards'
import { readImageDimensions } from '../../../packages/rari/src/vite/image/dimensions'
import {
  beginStaticImageSourceMapBuild,
  buildStaticImageModule,
  createStaticImageRolldownPlugin,
  finalizeStaticImageSourceMapBuild,
  isStaticImageModuleId,
  resolveStaticImageFilePath,
  resolveStaticImageOutDir,
} from '../../../packages/rari/src/vite/image/static-import'

function pngFixture(width: number, height: number): Buffer {
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
  const ihdrLength = Buffer.alloc(4)
  ihdrLength.writeUInt32BE(13)
  const ihdrType = Buffer.from('IHDR')
  const ihdrData = Buffer.alloc(13)
  ihdrData.writeUInt32BE(width, 0)
  ihdrData.writeUInt32BE(height, 4)
  ihdrData[8] = 8
  ihdrData[9] = 2
  const crc = Buffer.alloc(4)
  return Buffer.concat([signature, ihdrLength, ihdrType, ihdrData, crc])
}

function isoBox(type: string, payload: Buffer, fullBox = false): Buffer {
  const versionFlags = fullBox ? Buffer.alloc(4) : Buffer.alloc(0)
  const header = Buffer.alloc(8)
  header.writeUInt32BE(8 + versionFlags.length + payload.length, 0)
  header.write(type, 4, 4, 'ascii')
  return Buffer.concat([header, versionFlags, payload])
}

function ispeBox(width: number, height: number): Buffer {
  const payload = Buffer.alloc(8)
  payload.writeUInt32BE(width, 0)
  payload.writeUInt32BE(height, 4)
  return isoBox('ispe', payload, true)
}

function avifFixture(width: number, height: number): Buffer {
  const ftyp = Buffer.alloc(24)
  ftyp.writeUInt32BE(24, 0)
  ftyp.write('ftyp', 4, 4, 'ascii')
  ftyp.write('avif', 8, 4, 'ascii')

  const ipco = isoBox('ipco', ispeBox(width, height))
  const iprp = isoBox('iprp', ipco)
  const meta = isoBox('meta', iprp, true)
  return Buffer.concat([ftyp, meta])
}

function avifPrimaryItemFixture(
  primaryWidth: number,
  primaryHeight: number,
  auxWidth: number,
  auxHeight: number,
): Buffer {
  const ftyp = Buffer.alloc(24)
  ftyp.writeUInt32BE(24, 0)
  ftyp.write('ftyp', 4, 4, 'ascii')
  ftyp.write('avif', 8, 4, 'ascii')

  const pitmPayload = Buffer.alloc(2)
  pitmPayload.writeUInt16BE(1, 0)
  const pitm = isoBox('pitm', pitmPayload, true)

  const ipco = isoBox(
    'ipco',
    Buffer.concat([ispeBox(auxWidth, auxHeight), ispeBox(primaryWidth, primaryHeight)]),
  )

  const ipmaPayload = Buffer.alloc(12)
  ipmaPayload.writeUInt32BE(2, 0)
  ipmaPayload.writeUInt16BE(1, 4)
  ipmaPayload[6] = 1
  ipmaPayload[7] = 2
  ipmaPayload.writeUInt16BE(2, 8)
  ipmaPayload[10] = 1
  ipmaPayload[11] = 1
  const ipma = isoBox('ipma', ipmaPayload, true)

  const iprp = isoBox('iprp', Buffer.concat([ipco, ipma]))
  const meta = isoBox('meta', Buffer.concat([pitm, iprp]), true)
  return Buffer.concat([ftyp, meta])
}

function jpegFixture(width: number, height: number): Buffer {
  const buffer = Buffer.alloc(15)
  buffer[0] = 0xff
  buffer[1] = 0xd8
  buffer[2] = 0xff
  buffer[3] = 0xc0
  buffer.writeUInt16BE(11, 4)
  buffer[6] = 8
  buffer.writeUInt16BE(height, 7)
  buffer.writeUInt16BE(width, 9)
  buffer[11] = 1
  buffer[12] = 1
  buffer[13] = 0x11
  buffer[14] = 0
  return buffer
}

function malformedJpegSofFixture(): Buffer {
  const buffer = Buffer.alloc(12)
  buffer[0] = 0xff
  buffer[1] = 0xd8
  buffer[2] = 0xff
  buffer[3] = 0xc0
  buffer.writeUInt16BE(8, 4)
  buffer[6] = 8
  buffer.writeUInt16BE(240, 7)
  buffer.writeUInt16BE(320, 9)
  buffer[11] = 1
  return buffer
}

function zeroDimensionJpegFixture(): Buffer {
  return jpegFixture(0, 240)
}

function vp8lFixture(width: number, height: number, signature = 0x2f): Buffer {
  const buffer = Buffer.alloc(25)
  buffer.write('RIFF', 0, 4, 'ascii')
  buffer.writeUInt32LE(17, 4)
  buffer.write('WEBP', 8, 4, 'ascii')
  buffer.write('VP8L', 12, 4, 'ascii')
  buffer.writeUInt32LE(5, 16)
  buffer[20] = signature
  const bits = (width - 1) | ((height - 1) << 14)
  buffer.writeUInt32LE(bits, 21)
  return buffer
}

describe('readImageDimensions', () => {
  it('reads PNG dimensions', () => {
    expect(readImageDimensions(pngFixture(1200, 600))).toEqual({ width: 1200, height: 600 })
  })

  it('returns null for unrecognized buffers', () => {
    expect(readImageDimensions(Buffer.from('not-an-image'))).toBeNull()
  })

  it('reads AVIF dimensions from nested ispe', () => {
    expect(readImageDimensions(avifFixture(1920, 1080))).toEqual({ width: 1920, height: 1080 })
  })

  it('reads primary-item AVIF ispe when an auxiliary ispe comes first', () => {
    expect(readImageDimensions(avifPrimaryItemFixture(1920, 1080, 64, 64))).toEqual({
      width: 1920,
      height: 1080,
    })
  })

  it('reads JPEG SOF dimensions from a complete SOF segment', () => {
    expect(readImageDimensions(jpegFixture(320, 240))).toEqual({ width: 320, height: 240 })
  })

  it('rejects truncated JPEG SOF segments', () => {
    expect(readImageDimensions(malformedJpegSofFixture())).toBeNull()
  })

  it('rejects JPEG SOF with zero width or height', () => {
    expect(readImageDimensions(zeroDimensionJpegFixture())).toBeNull()
    expect(readImageDimensions(jpegFixture(320, 0))).toBeNull()
  })

  it('reads VP8L dimensions when signature is valid', () => {
    expect(readImageDimensions(vp8lFixture(400, 300))).toEqual({ width: 400, height: 300 })
  })

  it('rejects malformed VP8L without 0x2f signature', () => {
    expect(readImageDimensions(vp8lFixture(400, 300, 0x00))).toBeNull()
  })
})

describe('buildStaticImageModule', () => {
  it('emits hashed /assets URL and StaticImageData fields', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const filePath = path.join(dir, 'hero.png')
    fs.writeFileSync(filePath, pngFixture(800, 400))

    const built = buildStaticImageModule(filePath)
    expect(built.publicPath).toMatch(/^\/assets\/hero-[a-f0-9]{8}\.png$/)
    expect(built.code).toContain(`src = ${JSON.stringify(built.publicPath)}`)
    expect(built.code).toContain('width: 800')
    expect(built.code).toContain('height: 400')
  })

  it('emits StaticImageData for AVIF with nested ispe', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const filePath = path.join(dir, 'hero.avif')
    fs.writeFileSync(filePath, avifFixture(640, 480))

    const built = buildStaticImageModule(filePath)
    expect(built.publicPath).toMatch(/^\/assets\/hero-[a-f0-9]{8}\.avif$/)
    expect(built.code).toContain('width: 640')
    expect(built.code).toContain('height: 480')
  })

  it('emits primary-item dimensions for multi-ispe AVIF', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const filePath = path.join(dir, 'hero.avif')
    fs.writeFileSync(filePath, avifPrimaryItemFixture(1280, 720, 32, 32))

    const built = buildStaticImageModule(filePath)
    expect(built.code).toContain('width: 1280')
    expect(built.code).toContain('height: 720')
  })

  it('rejects malformed VP8L imports', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const filePath = path.join(dir, 'bad.webp')
    fs.writeFileSync(filePath, vp8lFixture(100, 100, 0x00))

    expect(() => buildStaticImageModule(filePath)).toThrow(/Unable to read dimensions/)
  })

  it('rejects truncated JPEG SOF imports', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const filePath = path.join(dir, 'bad.jpg')
    fs.writeFileSync(filePath, malformedJpegSofFixture())

    expect(() => buildStaticImageModule(filePath)).toThrow(/Unable to read dimensions/)
  })

  it('rejects JPEG imports with zero dimensions', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const filePath = path.join(dir, 'zero.jpg')
    fs.writeFileSync(filePath, zeroDimensionJpegFixture())

    expect(() => buildStaticImageModule(filePath)).toThrow(/Unable to read dimensions/)
  })

  it('url-encodes publicPath but keeps raw fileName for spaces', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const filePath = path.join(dir, 'my photo.png')
    fs.writeFileSync(filePath, pngFixture(50, 50))

    const built = buildStaticImageModule(filePath)
    expect(built.publicPath).toMatch(/^\/assets\/my%20photo-[a-f0-9]{8}\.png$/)
    expect(built.fileName).toMatch(/^assets\/my photo-[a-f0-9]{8}\.png$/)
  })

  it('url-encodes percent and unicode bases while keeping raw fileName', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))

    const percentPath = path.join(dir, '100%.png')
    fs.writeFileSync(percentPath, pngFixture(16, 16))
    const percentBuilt = buildStaticImageModule(percentPath)
    expect(percentBuilt.publicPath).toMatch(/^\/assets\/100%25-[a-f0-9]{8}\.png$/)
    expect(percentBuilt.fileName).toMatch(/^assets\/100%-[a-f0-9]{8}\.png$/)

    const unicodePath = path.join(dir, '写真.png')
    fs.writeFileSync(unicodePath, pngFixture(16, 16))
    const unicodeBuilt = buildStaticImageModule(unicodePath)
    expect(unicodeBuilt.publicPath).toMatch(
      new RegExp(`^/assets/${encodeURIComponent('写真')}-[a-f0-9]{8}\\.png$`),
    )
    expect(unicodeBuilt.fileName).toMatch(/^assets\/写真-[a-f0-9]{8}\.png$/)
  })

  it('records encoded public paths for spaces, percent, and Unicode in the source map', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const srcDir = path.join(dir, 'src')
    fs.mkdirSync(srcDir, { recursive: true })
    const names = ['my photo.png', '100%.png', '写真.png'] as const
    const plugin = createStaticImageRolldownPlugin(dir)

    for (const name of names) {
      const filePath = path.join(srcDir, name)
      fs.writeFileSync(filePath, pngFixture(8, 8))
      const resolved = plugin.resolveId(`./${name}`, path.join(srcDir, 'Page.tsx'))
      expect(resolved).toBeTruthy()
      plugin.load(resolved!)
    }

    const mapPath = path.join(dir, 'dist', 'server', 'static-image-sources.json')
    const map = parseJsonRecord(fs.readFileSync(mapPath, 'utf8'))
    expect(map).not.toBeNull()
    if (map == null) return

    for (const name of names) {
      const filePath = path.join(srcDir, name)
      const built = buildStaticImageModule(filePath)
      expect(map[built.publicPath]).toBe(filePath)
      expect(built.publicPath).toContain(encodeURIComponent(path.basename(name, '.png')))
      expect(decodeURIComponent(built.publicPath)).toContain(path.basename(name, '.png'))
    }
  })

  it('uses a custom assetsDir in the public path', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const filePath = path.join(dir, 'hero.png')
    fs.writeFileSync(filePath, pngFixture(100, 50))

    const built = buildStaticImageModule(filePath, 'static')
    expect(built.publicPath).toMatch(/^\/static\/hero-[a-f0-9]{8}\.png$/)
  })
})

describe('isStaticImageModuleId', () => {
  it('intercepts plain image imports', () => {
    expect(isStaticImageModuleId('./hero.png')).toBe(true)
  })

  it.each(['raw', 'url', 'inline', 'no-inline'] as const)(
    'leaves Vite-native ?%s imports alone',
    query => {
      expect(isStaticImageModuleId(`./hero.png?${query}`)).toBe(false)
      expect(isStaticImageModuleId(`./hero.png?${query}=1`)).toBe(false)
    },
  )

  it('still bypasses rari-static-bypass', () => {
    expect(isStaticImageModuleId('./hero.png?rari-static-bypass')).toBe(false)
  })
})

describe('resolveStaticImageFilePath', () => {
  it('resolves aliased image imports', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const srcDir = path.join(dir, 'src')
    const assetsDir = path.join(srcDir, 'assets')
    fs.mkdirSync(assetsDir, { recursive: true })
    const filePath = path.join(assetsDir, 'hero.png')
    fs.writeFileSync(filePath, pngFixture(32, 32))

    const resolved = resolveStaticImageFilePath(
      '@/assets/hero.png',
      path.join(srcDir, 'app/page.tsx'),
      dir,
      { '@': srcDir },
    )
    expect(resolved).toBe(filePath)
  })

  it('resolves relative imports from virtual SSR importers', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const componentPath = path.join(dir, 'src', 'Button.tsx')
    const filePath = path.join(dir, 'src', 'hero.png')
    fs.mkdirSync(path.dirname(componentPath), { recursive: true })
    fs.writeFileSync(filePath, pngFixture(16, 16))

    const resolved = resolveStaticImageFilePath('./hero.png', `\0ssr-virtual:${componentPath}`, dir)
    expect(resolved).toBe(filePath)
  })

  it('does not resolve Vite-native image queries', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const componentPath = path.join(dir, 'src', 'Button.tsx')
    const filePath = path.join(dir, 'src', 'hero.png')
    fs.mkdirSync(path.dirname(componentPath), { recursive: true })
    fs.writeFileSync(filePath, pngFixture(16, 16))

    expect(resolveStaticImageFilePath('./hero.png?raw', componentPath, dir)).toBeNull()
    expect(resolveStaticImageFilePath('./hero.png?url', componentPath, dir)).toBeNull()
    expect(resolveStaticImageFilePath('./hero.png?inline', componentPath, dir)).toBeNull()
    expect(resolveStaticImageFilePath('./hero.png?no-inline', componentPath, dir)).toBeNull()
  })
})

describe('createStaticImageRolldownPlugin', () => {
  it('handles aliased imports during SSR-style resolution', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const srcDir = path.join(dir, 'src')
    const assetsDir = path.join(srcDir, 'assets')
    fs.mkdirSync(assetsDir, { recursive: true })
    const filePath = path.join(assetsDir, 'hero.png')
    fs.writeFileSync(filePath, pngFixture(64, 64))

    const plugin = createStaticImageRolldownPlugin(dir, 'assets', { '@': srcDir })
    const resolved = plugin.resolveId('@/assets/hero.png', path.join(srcDir, 'Client.tsx'))
    expect(resolved).toBe(`\0rari-static-image:${filePath}`)

    const loaded = plugin.load(resolved!)
    expect(loaded?.code).toContain('width: 64')
    expect(loaded?.code).toContain('height: 64')
    expect(loaded?.code).toMatch(/\/assets\/hero-[a-f0-9]{8}\.png/)
  })

  it('merges source-map entries across plugin instances', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const srcDir = path.join(dir, 'src')
    fs.mkdirSync(srcDir, { recursive: true })
    const aPath = path.join(srcDir, 'a.png')
    const bPath = path.join(srcDir, 'b.png')
    fs.writeFileSync(aPath, pngFixture(10, 10))
    fs.writeFileSync(bPath, pngFixture(20, 20))

    const pluginA = createStaticImageRolldownPlugin(dir)
    const pluginB = createStaticImageRolldownPlugin(dir)
    const resolvedA = pluginA.resolveId('./a.png', path.join(srcDir, 'A.tsx'))
    const resolvedB = pluginB.resolveId('./b.png', path.join(srcDir, 'B.tsx'))
    expect(resolvedA).toBeTruthy()
    expect(resolvedB).toBeTruthy()

    pluginA.load(resolvedA!)
    pluginB.load(resolvedB!)

    const mapPath = path.join(dir, 'dist', 'server', 'static-image-sources.json')
    const map = parseJsonRecord(fs.readFileSync(mapPath, 'utf8'))
    expect(map).not.toBeNull()
    if (map == null) return

    const values = Object.values(map).filter(value => typeof value === 'string')
    expect(values).toContain(aPath)
    expect(values).toContain(bPath)
  })

  it('prunes deleted sources and superseded content hashes from the source map', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const srcDir = path.join(dir, 'src')
    fs.mkdirSync(srcDir, { recursive: true })
    const keepPath = path.join(srcDir, 'keep.png')
    const dropPath = path.join(srcDir, 'drop.png')
    const rehashPath = path.join(srcDir, 'rehash.png')
    fs.writeFileSync(keepPath, pngFixture(10, 10))
    fs.writeFileSync(dropPath, pngFixture(12, 12))
    fs.writeFileSync(rehashPath, pngFixture(14, 14))

    const plugin = createStaticImageRolldownPlugin(dir)
    const importer = path.join(srcDir, 'Page.tsx')
    const resolvedKeep = plugin.resolveId('./keep.png', importer)!
    const resolvedDrop = plugin.resolveId('./drop.png', importer)!
    const resolvedRehash = plugin.resolveId('./rehash.png', importer)!

    plugin.load(resolvedKeep)
    plugin.load(resolvedDrop)
    plugin.load(resolvedRehash)

    const mapPath = path.join(dir, 'dist', 'server', 'static-image-sources.json')
    const before = parseJsonRecord(fs.readFileSync(mapPath, 'utf8'))
    expect(before).not.toBeNull()
    if (before == null) return

    const dropPublicPath = buildStaticImageModule(dropPath).publicPath
    const oldRehashPublicPath = buildStaticImageModule(rehashPath).publicPath
    expect(before[dropPublicPath]).toBe(dropPath)
    expect(before[oldRehashPublicPath]).toBe(rehashPath)

    fs.unlinkSync(dropPath)
    fs.writeFileSync(rehashPath, pngFixture(30, 30))
    plugin.load(resolvedKeep)
    plugin.load(resolvedRehash)

    const after = parseJsonRecord(fs.readFileSync(mapPath, 'utf8'))
    expect(after).not.toBeNull()
    if (after == null) return

    const keepPublicPath = buildStaticImageModule(keepPath).publicPath
    const newRehashPublicPath = buildStaticImageModule(rehashPath).publicPath
    expect(after[keepPublicPath]).toBe(keepPath)
    expect(after[dropPublicPath]).toBeUndefined()
    expect(after[oldRehashPublicPath]).toBeUndefined()
    expect(after[newRehashPublicPath]).toBe(rehashPath)
    expect(oldRehashPublicPath).not.toBe(newRehashPublicPath)
  })

  it('removes images omitted from a subsequent complete build', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const srcDir = path.join(dir, 'src')
    fs.mkdirSync(srcDir, { recursive: true })
    const keepPath = path.join(srcDir, 'keep.png')
    const omitPath = path.join(srcDir, 'omit.png')
    fs.writeFileSync(keepPath, pngFixture(8, 8))
    fs.writeFileSync(omitPath, pngFixture(9, 9))

    const importer = path.join(srcDir, 'Page.tsx')
    const mapPath = path.join(dir, 'dist', 'server', 'static-image-sources.json')

    beginStaticImageSourceMapBuild(resolveStaticImageOutDir(dir))
    const firstBuild = createStaticImageRolldownPlugin(dir)
    firstBuild.load(firstBuild.resolveId('./keep.png', importer)!)
    firstBuild.load(firstBuild.resolveId('./omit.png', importer)!)
    finalizeStaticImageSourceMapBuild(resolveStaticImageOutDir(dir))

    const firstMap = parseJsonRecord(fs.readFileSync(mapPath, 'utf8'))
    expect(firstMap).not.toBeNull()
    if (firstMap == null) return
    const keepPublicPath = buildStaticImageModule(keepPath).publicPath
    const omitPublicPath = buildStaticImageModule(omitPath).publicPath
    expect(firstMap[keepPublicPath]).toBe(keepPath)
    expect(firstMap[omitPublicPath]).toBe(omitPath)

    beginStaticImageSourceMapBuild(resolveStaticImageOutDir(dir))
    const secondBuild = createStaticImageRolldownPlugin(dir)
    secondBuild.load(secondBuild.resolveId('./keep.png', importer)!)
    finalizeStaticImageSourceMapBuild(resolveStaticImageOutDir(dir))

    const secondMap = parseJsonRecord(fs.readFileSync(mapPath, 'utf8'))
    expect(secondMap).not.toBeNull()
    if (secondMap == null) return
    expect(secondMap[keepPublicPath]).toBe(keepPath)
    expect(secondMap[omitPublicPath]).toBeUndefined()
    expect(fs.existsSync(omitPath)).toBe(true)
  })

  it('keeps build-session entries across idempotent begin calls', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const srcDir = path.join(dir, 'src')
    fs.mkdirSync(srcDir, { recursive: true })
    const aPath = path.join(srcDir, 'a.png')
    const bPath = path.join(srcDir, 'b.png')
    fs.writeFileSync(aPath, pngFixture(4, 4))
    fs.writeFileSync(bPath, pngFixture(5, 5))

    const importer = path.join(srcDir, 'Page.tsx')
    const mapPath = path.join(dir, 'dist', 'server', 'static-image-sources.json')

    beginStaticImageSourceMapBuild(resolveStaticImageOutDir(dir))
    const pluginA = createStaticImageRolldownPlugin(dir)
    pluginA.load(pluginA.resolveId('./a.png', importer)!)

    beginStaticImageSourceMapBuild(resolveStaticImageOutDir(dir))
    const pluginB = createStaticImageRolldownPlugin(dir)
    pluginB.load(pluginB.resolveId('./b.png', importer)!)
    finalizeStaticImageSourceMapBuild(resolveStaticImageOutDir(dir))

    const map = parseJsonRecord(fs.readFileSync(mapPath, 'utf8'))
    expect(map).not.toBeNull()
    if (map == null) return
    expect(map[buildStaticImageModule(aPath).publicPath]).toBe(aPath)
    expect(map[buildStaticImageModule(bPath).publicPath]).toBe(bPath)
  })

  it('writes assets and source map under a custom outDir', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-static-image-'))
    const srcDir = path.join(dir, 'src')
    const customOutDir = path.join(dir, 'build-output')
    fs.mkdirSync(srcDir, { recursive: true })
    const filePath = path.join(srcDir, 'hero.png')
    fs.writeFileSync(filePath, pngFixture(24, 24))

    const plugin = createStaticImageRolldownPlugin(dir, 'assets', {}, customOutDir)
    const resolved = plugin.resolveId('./hero.png', path.join(srcDir, 'Page.tsx'))
    expect(resolved).toBeTruthy()
    plugin.load(resolved!)

    const built = buildStaticImageModule(filePath)
    expect(fs.existsSync(path.join(customOutDir, built.fileName))).toBe(true)
    expect(fs.existsSync(path.join(dir, 'dist', built.fileName))).toBe(false)

    const mapPath = path.join(customOutDir, 'server', 'static-image-sources.json')
    expect(fs.existsSync(mapPath)).toBe(true)
    expect(fs.existsSync(path.join(dir, 'dist', 'server', 'static-image-sources.json'))).toBe(false)

    const map = parseJsonRecord(fs.readFileSync(mapPath, 'utf8'))
    expect(map).not.toBeNull()
    if (map == null) return
    expect(map[built.publicPath]).toBe(filePath)
  })
})
