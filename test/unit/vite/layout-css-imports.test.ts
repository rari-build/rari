import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { buildLayoutCssImportStatements } from '@rari/vite/client-head'
import { describe, expect, it } from 'vite-plus/test'

describe('layout-css-imports', () => {
  it('collects css imports from layout files', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-layout-css-'))
    const appDir = path.join(dir, 'src', 'app')
    fs.mkdirSync(appDir, { recursive: true })
    fs.writeFileSync(path.join(appDir, 'globals.css'), 'body{}')
    fs.writeFileSync(
      path.join(appDir, 'layout.tsx'),
      `import './globals.css'\nexport default function Layout({ children }) { return children }\n`,
    )

    const statements = buildLayoutCssImportStatements(dir)
    expect(statements).toContain(`import "/src/app/globals.css";`)

    fs.rmSync(dir, { recursive: true, force: true })
  })

  it('returns empty string when layouts have no css imports', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-layout-css-empty-'))
    const appDir = path.join(dir, 'src', 'app')
    fs.mkdirSync(appDir, { recursive: true })
    fs.writeFileSync(
      path.join(appDir, 'layout.tsx'),
      `export default function Layout({ children }) { return children }\n`,
    )

    expect(buildLayoutCssImportStatements(dir)).toBe('')

    fs.rmSync(dir, { recursive: true, force: true })
  })
})
