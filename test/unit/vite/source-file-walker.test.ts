import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { collectSourceFilePaths, normalizeScanDirs } from '@rari/vite/analysis/source-walker'
import { describe, expect, it } from 'vite-plus/test'

describe('source-file-walker', () => {
  it('collects unique tsx files across directories', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-walk-'))
    const srcDir = path.join(dir, 'src')
    const componentsDir = path.join(srcDir, 'components')

    fs.mkdirSync(componentsDir, { recursive: true })
    fs.writeFileSync(path.join(srcDir, 'page.tsx'), 'export default function Page() {}')
    fs.writeFileSync(path.join(componentsDir, 'Button.tsx'), 'export default function Button() {}')
    fs.mkdirSync(path.join(srcDir, 'node_modules', 'pkg'), { recursive: true })
    fs.writeFileSync(path.join(srcDir, 'node_modules', 'pkg', 'index.tsx'), 'export default 1')

    const paths = collectSourceFilePaths(normalizeScanDirs(srcDir))

    expect(paths).toHaveLength(2)
    expect(paths.some(filePath => filePath.endsWith('page.tsx'))).toBe(true)
    expect(paths.some(filePath => filePath.endsWith('Button.tsx'))).toBe(true)

    fs.rmSync(dir, { recursive: true, force: true })
  })

  it('skips generated and metadata directories', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-walk-'))
    const srcDir = path.join(dir, 'src')

    fs.mkdirSync(path.join(srcDir, 'dist', 'nested'), { recursive: true })
    fs.mkdirSync(path.join(srcDir, '.git', 'nested'), { recursive: true })
    fs.writeFileSync(path.join(srcDir, 'page.tsx'), 'export default function Page() {}')
    fs.writeFileSync(path.join(srcDir, 'dist', 'nested', 'ignored.tsx'), 'export default 1')
    fs.writeFileSync(path.join(srcDir, '.git', 'nested', 'ignored.tsx'), 'export default 1')

    const paths = collectSourceFilePaths([srcDir])

    expect(paths).toEqual([path.join(srcDir, 'page.tsx')])

    fs.rmSync(dir, { recursive: true, force: true })
  })

  it('excludes additional dirs nested under the primary dir', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-walk-nested-'))
    const srcDir = path.join(dir, 'src')
    const nestedDir = path.join(srcDir, 'components')

    fs.mkdirSync(nestedDir, { recursive: true })
    fs.writeFileSync(path.join(srcDir, 'page.tsx'), 'export default function Page() {}')
    fs.writeFileSync(path.join(nestedDir, 'Button.tsx'), 'export default function Button() {}')

    const dirs = normalizeScanDirs(srcDir, [nestedDir])
    const paths = collectSourceFilePaths(dirs)

    expect(dirs).toEqual([srcDir])
    expect(paths).toHaveLength(2)

    fs.rmSync(dir, { recursive: true, force: true })
  })
})
