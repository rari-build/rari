/// <reference path="../types.d.ts" />

import { core } from 'ext:core/mod.js'

const { initializeDebugEnv } = core.loadExtScript('ext:deno_node/internal/util/debuglog.ts')

initializeDebugEnv('rari')

if (!g.process) {
  g.process = {
    env: {},
    cwd: () => {
      try {
        return g.Deno?.cwd() || '/'
      }
      catch {
        return '/'
      }
    },
    nextTick: (fn: () => void) => queueMicrotask(fn),
    platform: (() => {
      try {
        const os = g.Deno?.build?.os

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
        const arch = g.Deno?.build?.arch

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
    version: 'v__NODE_VERSION__',
    versions: {
      node: '__NODE_VERSION__',
      v8: '__V8_VERSION__',
    },
    argv: ['node'],
    execPath: (() => {
      try {
        return g.Deno?.execPath?.() || '/usr/bin/node'
      }
      catch {
        return '/usr/bin/node'
      }
    })(),
    execArgv: [],
    pid: (() => {
      try {
        return g.Deno?.pid ?? 1
      }
      catch {
        return 1
      }
    })(),
    ppid: (() => {
      try {
        return g.Deno?.ppid ?? 0
      }
      catch {
        return 0
      }
    })(),
    title: 'node',
    exit: (code: string | number | null = 0) => {
      if (g.Deno?.exit) {
        const exitCode = typeof code === 'number' ? code : 0
        g.Deno.exit(exitCode)
      }
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
      write: (data: string) => console.warn(data),
      isTTY: false,
    },
    stderr: {
      write: (data: string) => console.error(data),
      isTTY: false,
    },
    stdin: {
      isTTY: false,
    },
  }
}

if (!g.Buffer) {
  // @ts-expect-error - Minimal Buffer shim with incompatible signatures
  g.Buffer = class Buffer extends Uint8Array {
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

    static from(arg: string | ArrayLike<number> | ArrayBuffer | SharedArrayBuffer, encoding = 'utf8') {
      if (typeof arg === 'string') {
        let bytes

        const enc = encoding.toLowerCase().replace(/[-_]/g, '')

        switch (enc) {
          case 'base64': {
            const binaryString = atob(arg)
            bytes = new Uint8Array(binaryString.length)
            for (let i = 0; i < binaryString.length; i++) {
              bytes[i] = binaryString.charCodeAt(i)
            }
            break
          }
          case 'hex': {
            const hexStr = arg.replace(/\s/g, '')
            if (hexStr.length % 2 !== 0)
              throw new Error('Invalid hex string')
            bytes = new Uint8Array(hexStr.length / 2)
            for (let i = 0; i < hexStr.length; i += 2) {
              bytes[i / 2] = Number.parseInt(hexStr.slice(i, i + 2), 16)
            }
            break
          }
          case 'utf8':
          case 'utf-8':
          default: {
            bytes = new TextEncoder().encode(arg)
            break
          }
        }

        const buffer = new Uint8Array(bytes)
        Object.setPrototypeOf(buffer, Buffer.prototype)
        return buffer
      }

      if (arg instanceof Uint8Array || Array.isArray(arg)) {
        const buffer = new Uint8Array(arg)
        Object.setPrototypeOf(buffer, Buffer.prototype)
        return buffer
      }

      if (arg instanceof ArrayBuffer) {
        const buffer = new Uint8Array(arg)
        Object.setPrototypeOf(buffer, Buffer.prototype)
        return buffer
      }

      if (arg instanceof SharedArrayBuffer) {
        const buffer = new Uint8Array(arg)
        Object.setPrototypeOf(buffer, Buffer.prototype)
        return buffer
      }

      const buffer = new Uint8Array(arg as ArrayLike<number>)
      Object.setPrototypeOf(buffer, Buffer.prototype)
      return buffer
    }

    static alloc(size: number) {
      const buffer = new Uint8Array(size)
      Object.setPrototypeOf(buffer, Buffer.prototype)
      return buffer
    }

    static isBuffer(obj: unknown): obj is Buffer {
      return obj instanceof Buffer
    }
  }
}

if (!g.global)
  g.global = globalThis

if (!g.require) {
  const requireFn = function (specifier: string): never {
    throw new Error(
      `require('${specifier}') is not supported. Use ES modules: import ${specifier.replace(/^node:/, '')} from '${specifier}'`,
    )
  }
  requireFn.resolve = function (specifier: string) {
    return specifier
  }
  g.require = requireFn
}
