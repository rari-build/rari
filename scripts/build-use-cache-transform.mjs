#!/usr/bin/env node
// Build the use_cache_transform native addon via @napi-rs/cli.
//
// Usage:
//   node scripts/build-use-cache-transform.mjs                 # current platform (host)
//   node scripts/build-use-cache-transform.mjs --platform X    # target platform
//   node scripts/build-use-cache-transform.mjs --out-dir DIR   # where to place the .node
//
// When `--out-dir` is omitted, the file is written to:
//   packages/@rari/use-cache-transform/use_cache_transform.node   (dev mode)
//
// When `--platform` is given, the file is also written to the matching
// platform-specific package directory:
//   packages/@rari/use-cache-transform-{platform}/use_cache_transform.node

import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { copyFile, mkdir, rename, rm } from 'node:fs/promises'
import { resolve } from 'node:path'
import process from 'node:process'

const PROJECT_ROOT = resolve(import.meta.dirname, '..')
const MANIFEST_PATH = resolve(PROJECT_ROOT, 'crates/use-cache-transform/Cargo.toml')

const PLATFORM_TRIPLES = {
  'darwin-arm64': 'aarch64-apple-darwin',
  'darwin-x64': 'x86_64-apple-darwin',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'win32-arm64': 'aarch64-pc-windows-msvc',
  'win32-x64': 'x86_64-pc-windows-msvc',
}

// napi-rs uses its own triple suffix for the output filename, which differs
// from the cargo/rustup target triple passed via `--target`. We need the
// napi-rs one to locate the produced `.node`.
const NAPI_TRIPLES = {
  'darwin-arm64': 'darwin-arm64',
  'darwin-x64': 'darwin-x64',
  'linux-arm64': 'linux-arm64-gnu',
  'linux-x64': 'linux-x64-gnu',
  'win32-arm64': 'win32-arm64-msvc',
  'win32-x64': 'win32-x64-msvc',
}

function parseArgs(argv) {
  const out = { platform: null, outDir: null, dev: false, triple: null }
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]
    if (a === '--platform')
      out.platform = argv[++i]
    else if (a === '--out-dir')
      out.outDir = argv[++i]
    else if (a === '--dev')
      out.dev = true
    else if (a === '--target')
      out.triple = argv[++i]
  }

  return out
}

function detectHostPlatform() {
  const p = `${process.platform}-${process.arch}`
  if (!(p in PLATFORM_TRIPLES)) {
    throw new Error(`Unsupported host platform: ${p}`)
  }

  return p
}

function run(cmd, args, { cwd } = {}) {
  return new Promise((resolveP, rejectP) => {
    const child = spawn(cmd, args, { cwd, stdio: 'inherit', shell: process.platform === 'win32' })
    child.on('error', rejectP)
    child.on('exit', (code) => {
      if (code === 0)
        resolveP()
      else rejectP(new Error(`${cmd} exited with code ${code}`))
    })
  })
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const platform = args.platform ?? detectHostPlatform()
  const triple = args.triple ?? PLATFORM_TRIPLES[platform]
  if (!triple) {
    throw new Error(`No triple mapping for platform "${platform}". Use --target to specify.`)
  }
  const napiTriple = NAPI_TRIPLES[platform]
  if (!napiTriple) {
    throw new Error(`No napi triple mapping for platform "${platform}".`)
  }

  const tmpOut = resolve(PROJECT_ROOT, '.build/use-cache-transform')

  // napi-rs 3.x with --platform emits into --output-dir:
  //   index.js
  //   index.d.ts
  //   index.<napi-triple>.node
  // We only need the .node; the JS/.d.ts we ship ourselves.
  await mkdir(tmpOut, { recursive: true })

  const buildArgs = [
    'build',
    '--platform',
    '--js-package-name',
    '@rari/use-cache-transform',
    '--manifest-path',
    MANIFEST_PATH,
    '--output-dir',
    tmpOut,
    '--strip',
  ]
  if (!args.dev)
    buildArgs.push('--release')
  buildArgs.push('--', '--target', triple)

  console.log(`[build-use-cache-transform] running: napi ${buildArgs.join(' ')}`)
  await run('napi', buildArgs, { cwd: PROJECT_ROOT })

  const src = resolve(tmpOut, `index.${napiTriple}.node`)
  if (!existsSync(src)) {
    throw new Error(`expected artifact not found: ${src}`)
  }

  // Canonical location (always written): the main package directory.
  const mainPkgDir = resolve(PROJECT_ROOT, 'packages/@rari/use-cache-transform')
  await mkdir(mainPkgDir, { recursive: true })
  const canonical = resolve(mainPkgDir, 'use_cache_transform.node')
  await copyFile(src, canonical)

  const renamed = resolve(tmpOut, 'use_cache_transform.node')
  await rename(src, renamed)

  // When --out-dir is given, also copy there.
  if (args.outDir) {
    await mkdir(args.outDir, { recursive: true })
    await copyFile(renamed, resolve(args.outDir, 'use_cache_transform.node'))
  }

  // napi-rs also drops index.js / index.d.ts into the output dir; we ship our
  // own, so wipe them so the CI artifact stays minimal.
  await rm(resolve(tmpOut, 'index.js'), { force: true })
  await rm(resolve(tmpOut, 'index.d.ts'), { force: true })

  console.log(`[build-use-cache-transform] done (${platform}, ${triple})`)
}

main().catch((err) => {
  console.error('[build-use-cache-transform] failed:', err.message)
  process.exit(1)
})
