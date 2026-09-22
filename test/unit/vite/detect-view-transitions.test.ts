import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { detectViewTransitions } from '@rari/vite/view-transitions'
import { afterEach, describe, expect, it } from 'vite-plus/test'

describe('detectViewTransitions', () => {
  const dirs: string[] = []

  afterEach(() => {
    for (const dir of dirs) fs.rmSync(dir, { recursive: true, force: true })
    dirs.length = 0
  })

  function tempProject(css: string | null): string {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-vt-'))
    dirs.push(root)
    fs.mkdirSync(path.join(root, 'src', 'app'), { recursive: true })
    if (css != null) fs.writeFileSync(path.join(root, 'src', 'app', 'globals.css'), css)
    return root
  }

  it('is false when there is no VT CSS', () => {
    const root = tempProject('body { color: red; }')
    expect(detectViewTransitions(root)).toBe(false)
  })

  it('is true when app styles rari-page-vt', () => {
    const root = tempProject('::view-transition-new(.rari-page-vt) { opacity: 1; }')
    expect(detectViewTransitions(root)).toBe(true)
  })

  it('is true for ::view-transition-old markers', () => {
    const root = tempProject('::view-transition-old(root) { animation: none; }')
    expect(detectViewTransitions(root)).toBe(true)
  })
})
