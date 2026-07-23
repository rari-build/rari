import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { detectPackageManager } from '@rari/cli'
import { describe, expect, it } from 'vite-plus/test'

describe('cli project context', () => {
  it('detects bun from bun.lock', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-cli-'))
    fs.writeFileSync(path.join(dir, 'bun.lock'), '{}')

    const originalCwd = process.cwd()
    try {
      process.chdir(dir)
      expect(detectPackageManager()).toBe('bun')
    } finally {
      process.chdir(originalCwd)
      fs.rmSync(dir, { recursive: true, force: true })
    }
  })
})
