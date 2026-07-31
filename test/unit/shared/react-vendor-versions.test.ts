import fs from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import { isRecord } from '@rari/shared/utils/type-guards'
import { describe, expect, it } from 'vite-plus/test'

const require = createRequire(import.meta.url)
const TEST_DIR = path.dirname(fileURLToPath(import.meta.url))
const REPO_ROOT = path.resolve(TEST_DIR, '../../..')
const VENDOR_DIR = path.join(REPO_ROOT, 'crates/rari/src/runtime/ext/rari/react/vendor')
const MANIFEST_PATH = path.join(VENDOR_DIR, 'versions.json')

const VENDORED_PACKAGES = ['react', 'react-dom', 'react-server-dom-webpack'] as const

function readInstalledVersion(pkg: string): string {
  const pkgJsonPath = require.resolve(`${pkg}/package.json`)
  const parsed: unknown = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf-8'))
  if (!isRecord(parsed) || typeof parsed.version !== 'string')
    throw new Error(`Missing version field in ${pkgJsonPath}`)
  return parsed.version
}

const IS_CI = process.env.CI != null && process.env.CI !== '' && process.env.CI !== 'false'

describe('react vendor bundles', () => {
  it.skipIf(!IS_CI && !fs.existsSync(MANIFEST_PATH))(
    'generated vendors match the installed React package versions',
    () => {
      if (!fs.existsSync(MANIFEST_PATH)) {
        throw new Error(
          `Vendor manifest missing at ${MANIFEST_PATH}. Run \`just bundle-react-esm\` before testing.`,
        )
      }

      const manifest: unknown = JSON.parse(fs.readFileSync(MANIFEST_PATH, 'utf-8'))
      if (!isRecord(manifest)) throw new Error('versions.json is not an object')

      for (const pkg of VENDORED_PACKAGES) {
        expect({ pkg, version: manifest[pkg] }).toEqual({
          pkg,
          version: readInstalledVersion(pkg),
        })
      }
    },
  )
})
