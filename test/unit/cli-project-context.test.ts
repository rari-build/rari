import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { describe, expect, it } from 'vite-plus/test'
// eslint-disable-next-line antfu/no-import-dist
import { detectPackageManager } from '../../packages/rari/dist/cli.mjs'

describe('cli project context', () => {
  it('detects bun from bun.lock', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-cli-'))
    fs.writeFileSync(path.join(dir, 'bun.lock'), '{}')

    const originalCwd = process.cwd()
    try {
      process.chdir(dir)
      expect(detectPackageManager()).toBe('bun')
    }
    finally {
      process.chdir(originalCwd)
      fs.rmSync(dir, { recursive: true, force: true })
    }
  })
})
