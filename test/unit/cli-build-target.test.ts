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

  it('defaults configured build outDir to dist/client', () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-cli-default-out-'))
    try {
      expect(resolveConfiguredBuildOutDir(root)).toBe(path.join(root, 'dist', 'client'))
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  })

  it('finds image.json beside dist/client under dist/server', () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-cli-env-out-'))
    const clientOutDir = path.join(root, 'dist', 'client')
    const serverDir = path.join(root, 'dist', 'server')
    fs.mkdirSync(clientOutDir, { recursive: true })
    fs.mkdirSync(serverDir, { recursive: true })
    fs.writeFileSync(
      path.join(root, 'vite.config.ts'),
      `export default { build: { outDir: 'dist/client' } }\n`,
    )
    fs.writeFileSync(
      path.join(serverDir, 'image.json'),
      JSON.stringify({ assetsDir: 'assets', outDir: 'dist/client' }),
    )

    try {
      expect(resolveConfiguredBuildOutDir(root)).toBe(clientOutDir)
      const imageConfigPath = findImageConfigPath(root)
      expect(imageConfigPath).toBe(path.join(serverDir, 'image.json'))
      expect(outDirFromImageConfigPath(imageConfigPath!, root)).toBe(clientOutDir)
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  })

  it('finds image.json under defaultPackage ./frontend with dist/client', () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-cli-default-package-'))
    const frontend = path.join(root, 'frontend')
    const clientOutDir = path.join(frontend, 'dist', 'client')
    const serverDir = path.join(frontend, 'dist', 'server')
    fs.mkdirSync(clientOutDir, { recursive: true })
    fs.mkdirSync(serverDir, { recursive: true })
    fs.writeFileSync(
      path.join(root, 'vite.config.ts'),
      `export default { defaultPackage: './frontend' }\n`,
    )
    fs.writeFileSync(
      path.join(frontend, 'vite.config.ts'),
      `export default { build: { outDir: 'dist/client' } }\n`,
    )
    fs.writeFileSync(
      path.join(serverDir, 'image.json'),
      JSON.stringify({ assetsDir: 'assets', outDir: 'dist/client' }),
    )

    try {
      const packageRoot = resolveViteBuildPackageRoot(root, 'vp')
      expect(packageRoot).toBe(frontend)
      expect(resolveConfiguredBuildOutDir(packageRoot)).toBe(clientOutDir)

      const imageConfigPath = findImageConfigPath(packageRoot)
      expect(imageConfigPath).toBe(path.join(serverDir, 'image.json'))
      expect(outDirFromImageConfigPath(imageConfigPath!, packageRoot)).toBe(clientOutDir)

      expect(resolveViteBuildPackageRoot(root, 'vite')).toBe(root)
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  })

  it('resolves outDir against Vite root frontend with nested dist/client', () => {
    const repo = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-cli-vite-root-'))
    const frontend = path.join(repo, 'frontend')
    const clientOutDir = path.join(frontend, 'dist', 'client')
    const serverDir = path.join(frontend, 'dist', 'server')
    fs.mkdirSync(clientOutDir, { recursive: true })
    fs.mkdirSync(serverDir, { recursive: true })
    fs.writeFileSync(
      path.join(repo, 'vite.config.ts'),
      `export default { root: 'frontend', build: { outDir: 'dist/client' } }\n`,
    )
    fs.writeFileSync(
      path.join(serverDir, 'image.json'),
      JSON.stringify({ assetsDir: 'assets', outDir: 'dist/client' }),
    )

    try {
      expect(resolveConfiguredBuildOutDir(repo)).toBe(clientOutDir)

      const imageConfigPath = findImageConfigPath(repo)
      expect(imageConfigPath).toBe(path.join(serverDir, 'image.json'))
      expect(outDirFromImageConfigPath(imageConfigPath!, repo)).toBe(clientOutDir)
    } finally {
      fs.rmSync(repo, { recursive: true, force: true })
    }
  })

  it('resolves nested client outDir against package root, not dist parent', () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-cli-nested-out-'))
    const clientOutDir = path.join(root, 'output', 'web', 'client')
    const serverDir = path.join(root, 'output', 'web', 'server')
    fs.mkdirSync(clientOutDir, { recursive: true })
    fs.mkdirSync(serverDir, { recursive: true })
    fs.writeFileSync(
      path.join(root, 'vite.config.ts'),
      `export default { build: { outDir: 'output/web/client' } }\n`,
    )
    fs.writeFileSync(
      path.join(serverDir, 'image.json'),
      JSON.stringify({ assetsDir: 'assets', outDir: 'output/web/client' }),
    )

    try {
      const imageConfigPath = findImageConfigPath(root)
      expect(imageConfigPath).toBe(path.join(serverDir, 'image.json'))
      expect(outDirFromImageConfigPath(imageConfigPath!, root)).toBe(clientOutDir)
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  })
})
