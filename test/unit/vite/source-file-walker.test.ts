import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { parseHtmlEntryImports } from '@rari/vite/html-entry-imports'
import { collectSourceFilePaths, normalizeScanDirs } from '@rari/vite/source-file-walker'
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
})

describe('html-entry-imports', () => {
  it('parses html script imports under /src/', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-html-'))
    fs.writeFileSync(path.join(dir, 'index.html'), `<!doctype html>
<script type="module">import "/src/main.tsx"</script>
<script type="module">import("/src/app/page.tsx")</script>
`)

    const imports = parseHtmlEntryImports(dir)

    expect(imports.has(path.join(dir, 'src', 'main.tsx'))).toBe(true)
    expect(imports.has(path.join(dir, 'src', 'app', 'page.tsx'))).toBe(true)

    fs.rmSync(dir, { recursive: true, force: true })
  })
})
