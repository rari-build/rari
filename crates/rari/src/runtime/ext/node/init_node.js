/* eslint-disable node/prefer-global/buffer */
/* eslint-disable unused-imports/no-unused-vars */
/* eslint-disable node/prefer-global/process */
import { initializeDebugEnv } from 'ext:deno_node/internal/util/debuglog.ts'

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
      if (globalThis.Deno?.exit) {
        globalThis.Deno.exit(code)
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

const fs = {
  existsSync: (path) => {
    try {
      if (globalThis.Deno?.statSync) {
        globalThis.Deno.statSync(path)
        return true
      }
      return false
    }
    catch {
      return false
    }
  },
  readFileSync: (path, encoding = 'utf8') => {
    try {
      if (globalThis.Deno?.readTextFileSync) {
        return globalThis.Deno.readTextFileSync(path)
      }
      throw new Error('readFileSync not available')
    }
    catch (error) {
      throw new Error(`Cannot read file ${path}: ${error.message}`)
    }
  },
  writeFileSync: (path, data, encoding = 'utf8') => {
    try {
      if (globalThis.Deno?.writeTextFileSync) {
        return globalThis.Deno.writeTextFileSync(path, data)
      }
      throw new Error('writeFileSync not available')
    }
    catch (error) {
      throw new Error(`Cannot write file ${path}: ${error.message}`)
    }
  },
  readFile: (path, encoding, callback) => {
    if (typeof encoding === 'function') {
      callback = encoding
      encoding = 'utf8'
    }
    try {
      if (globalThis.Deno?.readTextFile) {
        globalThis.Deno.readTextFile(path).then(
          data => callback(null, data),
          err => callback(err),
        )
      }
      else {
        callback(new Error('readFile not available'))
      }
    }
    catch (error) {
      callback(error)
    }
  },
  writeFile: (path, data, encoding, callback) => {
    if (typeof encoding === 'function') {
      callback = encoding
      encoding = 'utf8'
    }
    try {
      if (globalThis.Deno?.writeTextFile) {
        globalThis.Deno.writeTextFile(path, data).then(
          () => callback(null),
          err => callback(err),
        )
      }
      else {
        callback(new Error('writeFile not available'))
      }
    }
    catch (error) {
      callback(error)
    }
  },
  promises: {
    readFile: async (path, encoding = 'utf8') => {
      try {
        if (globalThis.Deno?.readTextFile) {
          return await globalThis.Deno.readTextFile(path)
        }
        throw new Error('readFile not available')
      }
      catch (error) {
        throw new Error(`Cannot read file ${path}: ${error.message}`)
      }
    },
    writeFile: async (path, data, encoding = 'utf8') => {
      try {
        if (globalThis.Deno?.writeTextFile) {
          return await globalThis.Deno.writeTextFile(path, data)
        }
        throw new Error('writeFile not available')
      }
      catch (error) {
        throw new Error(`Cannot write file ${path}: ${error.message}`)
      }
    },
  },
}

const path = {
  join: (...paths) => {
    return paths.filter(Boolean).join('/').replace(/\/+/g, '/')
  },
  resolve: (...paths) => {
    let resolved = ''
    for (let i = paths.length - 1; i >= 0; i--) {
      const segment = paths[i]
      if (segment) {
        resolved = `${segment}/${resolved}`
        if (segment.startsWith('/')) {
          break
        }
      }
    }
    return resolved.replace(/\/+/g, '/').replace(/\/$/, '') || '/'
  },
  dirname: (path) => {
    const parts = path.split('/')
    parts.pop()
    return parts.join('/') || '/'
  },
  basename: (path, ext) => {
    let base = path.split('/').pop() || ''
    if (ext && base.endsWith(ext)) {
      base = base.slice(0, -ext.length)
    }
    return base
  },
  extname: (path) => {
    const base = path.split('/').pop() || ''
    const dot = base.lastIndexOf('.')
    return dot > 0 ? base.slice(dot) : ''
  },
  relative: (from, to) => {
    return to.replace(from, '').replace(/^\//, '')
  },
  isAbsolute: (path) => {
    return path.startsWith('/')
  },
  sep: '/',
  delimiter: ':',
  posix: {
    join: (...paths) => paths.filter(Boolean).join('/').replace(/\/+/g, '/'),
    resolve: (...paths) => {
      let resolved = ''
      for (let i = paths.length - 1; i >= 0; i--) {
        const segment = paths[i]
        if (segment) {
          resolved = `${segment}/${resolved}`
          if (segment.startsWith('/')) {
            break
          }
        }
      }
      return resolved.replace(/\/+/g, '/').replace(/\/$/, '') || '/'
    },
    dirname: (path) => {
      const parts = path.split('/')
      parts.pop()
      return parts.join('/') || '/'
    },
    basename: (path, ext) => {
      let base = path.split('/').pop() || ''
      if (ext && base.endsWith(ext)) {
        base = base.slice(0, -ext.length)
      }
      return base
    },
    extname: (path) => {
      const base = path.split('/').pop() || ''
      const dot = base.lastIndexOf('.')
      return dot > 0 ? base.slice(dot) : ''
    },
    sep: '/',
    delimiter: ':',
  },
}

const crypto = {
  createHash: (algorithm) => {
    if (globalThis.crypto?.subtle) {
      return {
        update: (data) => {
          return {
            digest: (encoding) => {
              return `hash_${btoa(data).slice(0, 32)}`
            },
          }
        },
      }
    }
    throw new Error('crypto.createHash not available')
  },
  randomBytes: (size) => {
    if (globalThis.crypto?.getRandomValues) {
      return globalThis.crypto.getRandomValues(new Uint8Array(size))
    }
    throw new Error('crypto.randomBytes not available')
  },
  randomUUID: () => {
    if (globalThis.crypto?.randomUUID) {
      return globalThis.crypto.randomUUID()
    }
    throw new Error('crypto.randomUUID not available')
  },
}

const util = {
  inspect: (obj, options = {}) => {
    try {
      return JSON.stringify(obj, null, options.depth || 2)
    }
    catch {
      return String(obj)
    }
  },
  format: (f, ...args) => {
    let index = 0
    const str = String(f).replace(/%[sdj%]/g, (x) => {
      if (index >= args.length)
        return x
      switch (x) {
        case '%s':
          return String(args[index++])
        case '%d':
          return Number(args[index++])
        case '%j':
          try {
            return JSON.stringify(args[index++])
          }
          catch {
            return '[Circular]'
          }
        default:
          return x
      }
    })
    return str
  },
  promisify: (fn) => {
    return function (...args) {
      return new Promise((resolve, reject) => {
        fn.call(this, ...args, (err, result) => {
          if (err)
            reject(err)
          else resolve(result)
        })
      })
    }
  },
}

const os = {
  platform: () => {
    try {
      const platform = globalThis.Deno?.build?.os
      if (platform === 'darwin')
        return 'darwin'
      if (platform === 'linux')
        return 'linux'
      if (platform === 'windows')
        return 'win32'
      return 'linux'
    }
    catch {
      return 'linux'
    }
  },
  arch: () => {
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
  },
  cpus: () => {
    try {
      return globalThis.navigator?.hardwareConcurrency || 4
    }
    catch {
      return 4
    }
  },
  homedir: () => {
    try {
      return (
        globalThis.Deno?.env?.get('HOME')
        || globalThis.Deno?.env?.get('USERPROFILE')
        || '/'
      )
    }
    catch {
      return '/'
    }
  },
  tmpdir: () => {
    try {
      return (
        globalThis.Deno?.env?.get('TMPDIR')
        || globalThis.Deno?.env?.get('TMP')
        || '/tmp'
      )
    }
    catch {
      return '/tmp'
    }
  },
  endianness: () => 'LE',
  EOL: '\n',
}

const Buffer = globalThis.Buffer || {
  from: (data, encoding = 'utf8') => {
    if (typeof data === 'string') {
      return new TextEncoder().encode(data)
    }
    return new Uint8Array(data)
  },
  alloc: (size, fill = 0) => {
    const buf = new Uint8Array(size)
    buf.fill(fill)
    return buf
  },
  isBuffer: (obj) => {
    return obj instanceof Uint8Array
  },
}

const EventEmitter = class {
  constructor() {
    this._events = {}
  }

  on(event, listener) {
    if (!this._events[event]) {
      this._events[event] = []
    }
    this._events[event].push(listener)
    return this
  }

  emit(event, ...args) {
    if (!this._events[event])
      return false
    this._events[event].forEach((listener) => {
      try {
        listener.apply(this, args)
      }
      catch (error) {
        console.error('EventEmitter error:', error)
      }
    })
    return true
  }

  removeListener(event, listener) {
    if (!this._events[event])
      return this
    this._events[event] = this._events[event].filter(l => l !== listener)
    return this
  }

  off(event, listener) {
    return this.removeListener(event, listener)
  }
}

const nodeModules = new Map([
  ['node:process', globalThis.process],
  ['process', globalThis.process],
  ['node:fs', fs],
  ['fs', fs],
  ['node:fs/promises', fs.promises],
  ['fs/promises', fs.promises],
  ['node:path', path],
  ['path', path],
  ['node:crypto', crypto],
  ['crypto', crypto],
  ['node:util', util],
  ['util', util],
  ['node:os', os],
  ['os', os],
  ['node:buffer', { Buffer }],
  ['buffer', { Buffer }],
  ['node:events', { EventEmitter }],
  ['events', { EventEmitter }],
  ['node:stream', {
    Stream: EventEmitter,
    Readable: EventEmitter,
    Writable: EventEmitter,
    Transform: EventEmitter,
    PassThrough: EventEmitter,
  }],
  ['stream', {
    Stream: EventEmitter,
    Readable: EventEmitter,
    Writable: EventEmitter,
    Transform: EventEmitter,
    PassThrough: EventEmitter,
  }],
  [
    'node:url',
    { URL: globalThis.URL, URLSearchParams: globalThis.URLSearchParams },
  ],
  [
    'url',
    { URL: globalThis.URL, URLSearchParams: globalThis.URLSearchParams },
  ],
  [
    'node:querystring',
    {
      parse: (str) => {
        const params = new URLSearchParams(str)
        const result = {}
        for (const [key, value] of params) {
          result[key] = value
        }
        return result
      },
      stringify: (obj) => {
        return new URLSearchParams(obj).toString()
      },
    },
  ],
  [
    'querystring',
    {
      parse: (str) => {
        const params = new URLSearchParams(str)
        const result = {}
        for (const [key, value] of params) {
          result[key] = value
        }
        return result
      },
      stringify: (obj) => {
        return new URLSearchParams(obj).toString()
      },
    },
  ],
  [
    'node:timers',
    {
      setTimeout: globalThis.setTimeout,
      clearTimeout: globalThis.clearTimeout,
      setInterval: globalThis.setInterval,
      clearInterval: globalThis.clearInterval,
      setImmediate: fn => setTimeout(fn, 0),
      clearImmediate: clearTimeout,
    },
  ],
  [
    'timers',
    {
      setTimeout: globalThis.setTimeout,
      clearTimeout: globalThis.clearTimeout,
      setInterval: globalThis.setInterval,
      clearInterval: globalThis.clearInterval,
      setImmediate: fn => setTimeout(fn, 0),
      clearImmediate: clearTimeout,
    },
  ],
  [
    'node:assert',
    {
      ok: (value, message) => {
        if (!value) {
          throw new Error(message || 'Assertion failed')
        }
      },
      equal: (actual, expected, message) => {
        if (actual !== expected) {
          throw new Error(message || `Expected ${expected}, got ${actual}`)
        }
      },
      deepEqual: (actual, expected, message) => {
        if (JSON.stringify(actual) !== JSON.stringify(expected)) {
          throw new Error(message || 'Deep equality assertion failed')
        }
      },
    },
  ],
  [
    'assert',
    {
      ok: (value, message) => {
        if (!value) {
          throw new Error(message || 'Assertion failed')
        }
      },
      equal: (actual, expected, message) => {
        if (actual !== expected) {
          throw new Error(message || `Expected ${expected}, got ${actual}`)
        }
      },
      deepEqual: (actual, expected, message) => {
        if (JSON.stringify(actual) !== JSON.stringify(expected)) {
          throw new Error(message || 'Deep equality assertion failed')
        }
      },
    },
  ],
  [
    'node:child_process',
    {
      spawn: () => {
        console.warn('child_process.spawn is not supported')
        return { on: () => {}, stdout: { on: () => {} }, stderr: { on: () => {} } }
      },
      exec: (cmd, cb) => {
        console.warn('child_process.exec is not supported')
        if (cb)
          cb(new Error('Not supported'), '', '')
      },
    },
  ],
  [
    'child_process',
    {
      spawn: () => {
        console.warn('child_process.spawn is not supported')
        return { on: () => {}, stdout: { on: () => {} }, stderr: { on: () => {} } }
      },
      exec: (cmd, cb) => {
        console.warn('child_process.exec is not supported')
        if (cb)
          cb(new Error('Not supported'), '', '')
      },
    },
  ],
  [
    'node:tty',
    {
      isatty: () => false,
      ReadStream: class {},
      WriteStream: class {},
    },
  ],
  [
    'tty',
    {
      isatty: () => false,
      ReadStream: class {},
      WriteStream: class {},
    },
  ],
  [
    'node:net',
    {
      createServer: () => {
        console.warn('net.createServer is not supported')
        return { listen: () => {}, on: () => {} }
      },
      connect: () => {
        console.warn('net.connect is not supported')
        return { on: () => {}, write: () => {}, end: () => {} }
      },
    },
  ],
  [
    'net',
    {
      createServer: () => {
        console.warn('net.createServer is not supported')
        return { listen: () => {}, on: () => {} }
      },
      connect: () => {
        console.warn('net.connect is not supported')
        return { on: () => {}, write: () => {}, end: () => {} }
      },
    },
  ],
  [
    'node:http',
    {
      createServer: () => {
        console.warn('http.createServer is not supported')
        return { listen: () => {}, on: () => {} }
      },
      request: () => {
        console.warn('http.request is not supported')
        return { on: () => {}, write: () => {}, end: () => {} }
      },
    },
  ],
  [
    'http',
    {
      createServer: () => {
        console.warn('http.createServer is not supported')
        return { listen: () => {}, on: () => {} }
      },
      request: () => {
        console.warn('http.request is not supported')
        return { on: () => {}, write: () => {}, end: () => {} }
      },
    },
  ],
  [
    'node:https',
    {
      createServer: () => {
        console.warn('https.createServer is not supported')
        return { listen: () => {}, on: () => {} }
      },
      request: () => {
        console.warn('https.request is not supported')
        return { on: () => {}, write: () => {}, end: () => {} }
      },
    },
  ],
  [
    'https',
    {
      createServer: () => {
        console.warn('https.createServer is not supported')
        return { listen: () => {}, on: () => {} }
      },
      request: () => {
        console.warn('https.request is not supported')
        return { on: () => {}, write: () => {}, end: () => {} }
      },
    },
  ],
  [
    'node:zlib',
    {
      createGzip: () => ({ pipe: x => x, on: () => {} }),
      createGunzip: () => ({ pipe: x => x, on: () => {} }),
    },
  ],
  [
    'zlib',
    {
      createGzip: () => ({ pipe: x => x, on: () => {} }),
      createGunzip: () => ({ pipe: x => x, on: () => {} }),
    },
  ],
  [
    'node:readline',
    {
      createInterface: () => ({ on: () => {}, close: () => {} }),
    },
  ],
  [
    'readline',
    {
      createInterface: () => ({ on: () => {}, close: () => {} }),
    },
  ],
  [
    'node:module',
    {
      createRequire: () => globalThis.require,
      builtinModules: ['fs', 'path', 'os', 'util', 'crypto', 'stream', 'buffer', 'events'],
    },
  ],
  [
    'module',
    {
      createRequire: () => globalThis.require,
      builtinModules: ['fs', 'path', 'os', 'util', 'crypto', 'stream', 'buffer', 'events'],
    },
  ],
])

if (!globalThis['~node'])
  globalThis['~node'] = {}
if (!globalThis['~node'].modules) {
  globalThis['~node'].modules = nodeModules
  const streamExports = nodeModules.get('node:stream')
}

if (globalThis.import) {
  const originalImport = globalThis.import
  globalThis.import = function (specifier) {
    if (nodeModules.has(specifier)) {
      return Promise.resolve({
        default: nodeModules.get(specifier),
        ...nodeModules.get(specifier),
      })
    }

    return originalImport.call(this, specifier).catch((error) => {
      if (globalThis['~rari']?.runtimeState) {
        globalThis['~rari'].runtimeState.import_errors.push({
          specifier,
          error: error.message,
          timestamp: Date.now(),
        })
      }
      throw new Error(`Failed to import ${specifier}: ${error.message}`)
    })
  }
}

if (!globalThis.Buffer) {
  globalThis.Buffer = Buffer
}

if (!globalThis.global) {
  globalThis.global = globalThis
}

if (!globalThis.require) {
  globalThis.require = function (specifier) {
    const nodeSpecifier = specifier.startsWith('node:') ? specifier : `node:${specifier}`

    if (nodeModules.has(nodeSpecifier)) {
      return nodeModules.get(nodeSpecifier)
    }

    if (nodeModules.has(specifier)) {
      return nodeModules.get(specifier)
    }

    throw new Error(`Cannot find module '${specifier}'`)
  }

  globalThis.require.resolve = function (specifier) {
    return specifier
  }
}
