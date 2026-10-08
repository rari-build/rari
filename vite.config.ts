import { existsSync } from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite-plus'
import { monorepoFmt, monorepoLint } from './.config/lint/monorepo'

const rootDir = process.cwd()
const reactSrc = path.join(rootDir, 'packages/react/src')
const coreSrc = path.join(rootDir, 'packages/core/src')
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

      if (importer.includes(`${path.sep}packages${path.sep}react${path.sep}`))
        return resolvePackageInternal(subpath, reactSrc)

      if (importer.includes(`${path.sep}packages${path.sep}core${path.sep}`))
        return resolvePackageInternal(subpath, coreSrc)

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
      // The CLI lives in the rari package; binary resolution moved to @rari/core.
      '@rari/cli/platform': fileURLToPath(
        new URL('./packages/core/src/platform.ts', import.meta.url),
      ),
      '@rari/cli': fileURLToPath(new URL('./packages/rari/src/cli/index.ts', import.meta.url)),
      // @rari/core subpaths mirror its src layout, so tests run against sources.
      '@rari/core/router': fileURLToPath(new URL('./packages/core/src/router', import.meta.url)),
      '@rari/core/regex-constants': fileURLToPath(
        new URL('./packages/core/src/regex-constants.ts', import.meta.url),
      ),
      '@rari/core/utils': fileURLToPath(new URL('./packages/core/src/utils', import.meta.url)),
      '@rari/core': fileURLToPath(new URL('./packages/core/src', import.meta.url)),
      // Tests address the React adapter's internals as `@rari/<dir>`.
      '@rari': fileURLToPath(new URL('./packages/react/src', import.meta.url)),
      '@rari/runtime': fileURLToPath(new URL('./packages/react/src/runtime', import.meta.url)),
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
  fmt: monorepoFmt,
  lint: monorepoLint,
})
