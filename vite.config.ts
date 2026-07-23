import { existsSync } from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import { fmt, lint } from '@rari/lint/vite'
import { defineConfig } from 'vite-plus'

const rootDir = process.cwd()
const rariSrc = path.join(rootDir, 'packages/rari/src')
const useCacheSrc = path.join(rootDir, 'packages/use-cache/src')

function resolvePackageInternal(subpath: string, baseDir: string) {
  const candidates = [
    path.join(baseDir, subpath),
    `${path.join(baseDir, subpath)}.ts`,
    path.join(baseDir, subpath, 'index.ts'),
  ]
  return candidates.find(candidate => existsSync(candidate)) ?? null
}

function packageInternalAlias() {
  return {
    name: 'package-internal-alias',
    enforce: 'pre' as const,
    resolveId(source: string, importer?: string) {
      if (!source.startsWith('@/') || importer == null || importer === '') return null

      const subpath = source.slice(2)
      if (importer.includes(`${path.sep}packages${path.sep}use-cache${path.sep}`))
        return resolvePackageInternal(subpath, useCacheSrc)

      if (importer.includes(`${path.sep}packages${path.sep}rari${path.sep}`))
        return resolvePackageInternal(subpath, rariSrc)

      return null
    },
  }
}

export default defineConfig({
  plugins: [packageInternalAlias()],
  resolve: {
    alias: {
      '@rari/use-cache/runtime/cache-wrapper': fileURLToPath(
        new URL('./packages/use-cache/dist/runtime/cache-wrapper.mjs', import.meta.url),
      ),
      '@rari/use-cache-darwin-arm64': fileURLToPath(
        new URL('./packages/use-cache-darwin-arm64', import.meta.url),
      ),
      '@rari/use-cache-darwin-x64': fileURLToPath(
        new URL('./packages/use-cache-darwin-x64', import.meta.url),
      ),
      '@rari/use-cache-linux-arm64': fileURLToPath(
        new URL('./packages/use-cache-linux-arm64', import.meta.url),
      ),
      '@rari/use-cache-linux-x64': fileURLToPath(
        new URL('./packages/use-cache-linux-x64', import.meta.url),
      ),
      '@rari/use-cache-win32-arm64': fileURLToPath(
        new URL('./packages/use-cache-win32-arm64', import.meta.url),
      ),
      '@rari/use-cache-win32-x64': fileURLToPath(
        new URL('./packages/use-cache-win32-x64', import.meta.url),
      ),
      '@rari/use-cache': fileURLToPath(new URL('./packages/use-cache/src', import.meta.url)),
      '@rari/logger': fileURLToPath(new URL('./packages/logger/src', import.meta.url)),
      '@rari': fileURLToPath(new URL('./packages/rari/src', import.meta.url)),
      '@rari/runtime': fileURLToPath(new URL('./packages/rari/src/runtime', import.meta.url)),
    },
  },
  test: {
    globals: true,
    include: ['test/**/*.test.ts'],
    setupFiles: ['./test/setup.ts'],
    coverage: {
      include: ['packages/*/src/**/*.{ts,tsx}'],
      exclude: ['node_modules', 'test', '**/*.config.ts', '**/dist'],
    },
  },
  fmt,
  lint,
})
