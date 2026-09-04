import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { describe, expect, it } from 'vite-plus/test'
import {
  findImageConfigPath,
  outDirFromImageConfigPath,
  readBuildOutDir,
  readDefaultPackageTarget,
  resolveConfiguredBuildOutDir,
  resolveViteBuildPackageRoot,
} from '../../packages/rari/src/cli/build-target'

describe('cli build target resolution', () => {
  it('reads string and per-command defaultPackage targets', () => {
    expect(readDefaultPackageTarget(`export default { defaultPackage: './frontend' }`)).toBe(
      './frontend',
    )
    expect(
      readDefaultPackageTarget(
        `export default { defaultPackage: { dev: './apps/web', build: './frontend' } }`,
        'build',
      ),
    ).toBe('./frontend')
    expect(
      readDefaultPackageTarget(
        `export default { defaultPackage: { dev: './apps/web', build: './frontend' } }`,
        'dev',
      ),
    ).toBe('./apps/web')
  })

  it('reads build.outDir from Vite config source', () => {
    expect(readBuildOutDir(`export default { build: { outDir: 'dist/client' } }`)).toBe(
      'dist/client',
    )
    expect(readBuildOutDir(`export default { outDir: 'custom-dist' }`)).toBe('custom-dist')
  })

  it('finds image.json under defaultPackage ./frontend with a custom outDir', () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-cli-default-package-'))
    const frontend = path.join(root, 'frontend')
    const customOutDir = path.join(frontend, 'custom-dist')
    fs.mkdirSync(path.join(customOutDir, 'server'), { recursive: true })
    fs.writeFileSync(
      path.join(root, 'vite.config.ts'),
      `export default { defaultPackage: './frontend' }\n`,
    )
    fs.writeFileSync(
      path.join(frontend, 'vite.config.ts'),
      `export default { build: { outDir: 'custom-dist' } }\n`,
    )
    fs.writeFileSync(
      path.join(customOutDir, 'server', 'image.json'),
      JSON.stringify({ assetsDir: 'assets', outDir: 'custom-dist' }),
    )

    try {
      const packageRoot = resolveViteBuildPackageRoot(root, 'vp')
      expect(packageRoot).toBe(frontend)
      expect(resolveConfiguredBuildOutDir(packageRoot)).toBe(customOutDir)

      const imageConfigPath = findImageConfigPath(packageRoot)
      expect(imageConfigPath).toBe(path.join(customOutDir, 'server', 'image.json'))
      expect(outDirFromImageConfigPath(imageConfigPath!)).toBe(customOutDir)

      expect(resolveViteBuildPackageRoot(root, 'vite')).toBe(root)
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  })

  it('finds image.json under nested dist/client outDir from config', () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-cli-nested-out-'))
    const nestedOutDir = path.join(root, 'dist', 'client')
    fs.mkdirSync(path.join(nestedOutDir, 'server'), { recursive: true })
    fs.writeFileSync(
      path.join(root, 'vite.config.ts'),
      `export default { build: { outDir: 'dist/client' } }\n`,
    )
    fs.writeFileSync(
      path.join(nestedOutDir, 'server', 'image.json'),
      JSON.stringify({ assetsDir: 'assets', outDir: 'dist/client' }),
    )

    try {
      expect(resolveConfiguredBuildOutDir(root)).toBe(nestedOutDir)
      const imageConfigPath = findImageConfigPath(root)
      expect(imageConfigPath).toBe(path.join(nestedOutDir, 'server', 'image.json'))
      expect(outDirFromImageConfigPath(imageConfigPath!)).toBe(nestedOutDir)
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  })

  it('resolves outDir against Vite root frontend with nested dist/client', () => {
    const repo = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-cli-vite-root-'))
    const frontend = path.join(repo, 'frontend')
    const nestedOutDir = path.join(frontend, 'dist', 'client')
    fs.mkdirSync(path.join(nestedOutDir, 'server'), { recursive: true })
    fs.writeFileSync(
      path.join(repo, 'vite.config.ts'),
      `export default { root: 'frontend', build: { outDir: 'dist/client' } }\n`,
    )
    fs.writeFileSync(
      path.join(nestedOutDir, 'server', 'image.json'),
      JSON.stringify({ assetsDir: 'assets', outDir: 'dist/client' }),
    )

    try {
      expect(resolveConfiguredBuildOutDir(repo)).toBe(nestedOutDir)

      const imageConfigPath = findImageConfigPath(repo)
      expect(imageConfigPath).toBe(path.join(nestedOutDir, 'server', 'image.json'))
      expect(outDirFromImageConfigPath(imageConfigPath!)).toBe(nestedOutDir)

      const configuredImageConfigPath = path.join(
        resolveConfiguredBuildOutDir(repo),
        'server',
        'image.json',
      )
      expect(fs.existsSync(configuredImageConfigPath)).toBe(true)
    } finally {
      fs.rmSync(repo, { recursive: true, force: true })
    }
  })
})
