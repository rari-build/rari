/* eslint-disable node/prefer-global/buffer, unused-imports/no-unused-vars, node/prefer-global/process */
import { core } from 'ext:core/mod.js'

const { initializeDebugEnv } = core.loadExtScript('ext:deno_node/internal/util/debuglog.ts')

initializeDebugEnv('rari')

if (!globalThis.process) {
  globalThis.process = {
    env: {},
    cwd: () => {
      try {
        return globalThis.Deno?.cwd() || '/'
      }
      catch {
        return '/'
      }
    },
    nextTick: fn => setTimeout(fn, 0),
    platform: (() => {
      try {
        const os = globalThis.Deno?.build?.os
        if (os === 'darwin')
          return 'darwin'
        if (os === 'linux')
          return 'linux'
        if (os === 'windows')
          return 'win32'

        return 'linux'
      }
      catch {
        return 'linux'
      }
    })(),
    arch: (() => {
      try {
        const arch = globalThis.Deno?.build?.arch
        if (arch === 'x86_64')
          return 'x64'
        if (arch === 'aarch64')
          return 'arm64'

        return 'x64'
      }
      catch {
        return 'x64'
      }
    })(),
    version: 'v20.0.0',
    versions: {
      node: '20.0.0',
      v8: '11.0.0',
      uv: '1.0.0',
      zlib: '1.0.0',
      modules: '108',
    },
    argv: ['node'],
    execPath: '/usr/bin/node',
    execArgv: [],
    pid: 1,
    ppid: 0,
    title: 'node',
    exit: (code = 0) => {
      if (globalThis.Deno?.exit)
        globalThis.Deno.exit(code)
    },
    kill: () => {},
    memoryUsage: () => ({
      rss: 0,
      heapTotal: 0,
      heapUsed: 0,
      external: 0,
      arrayBuffers: 0,
    }),
    uptime: () => 0,
    hrtime: () => [0, 0],
    binding: () => ({}),
    stdout: {
      write: data => console.warn(data),
      isTTY: false,
    },
    stderr: {
      write: data => console.error(data),
      isTTY: false,
    },
    stdin: {
      isTTY: false,
    },
  }
}

if (!globalThis.Buffer) {
  globalThis.Buffer = class Buffer extends Uint8Array {
    toString(encoding = 'utf8') {
      if (encoding === 'utf8' || encoding === 'utf-8') {
        return new TextDecoder().decode(this)
      }
      if (encoding === 'hex') {
        return Array.from(this).map(b => b.toString(16).padStart(2, '0')).join('')
      }
      if (encoding === 'base64') {
        return btoa(String.fromCharCode(...this))
      }

      return new TextDecoder().decode(this)
    }

    toJSON() {
      return { type: 'Buffer', data: Array.from(this) }
    }

    static from(arg, encoding) {
      if (typeof arg === 'string') {
        return Buffer.from(new TextEncoder().encode(arg))
      }

      return Buffer.from(arg)
    }

    static alloc(size) {
      return Buffer.alloc(size)
    }

    static isBuffer(obj) {
      return obj instanceof Buffer || obj instanceof Uint8Array
    }
  }
}

if (!globalThis.global)
  globalThis.global = globalThis

if (!globalThis.require) {
  globalThis.require = function (specifier) {
    throw new Error(
      `require('${specifier}') is not supported. Use ES modules: import ${specifier.replace(/^node:/, '')} from '${specifier}'`,
    )
  }

  globalThis.require.resolve = function (specifier) {
    return specifier
  }
}
