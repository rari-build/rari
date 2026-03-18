pub const NODE_FS_STUB: &str = r#"
// ESM-compatible bridge for node:fs to Deno APIs

const readFileSync = (path, encoding) => {
  try {
    if (globalThis.Deno?.readTextFileSync) {
      const content = globalThis.Deno.readTextFileSync(path);
      if (encoding === 'utf-8' || encoding === 'utf8') {
        return content;
      }
      return new TextEncoder().encode(content);
    }
    return new Uint8Array(0);
  } catch (error) {
    throw new Error(`ENOENT: no such file or directory, open '${path}'`);
  }
};

const readFile = async (path, encoding) => {
  try {
    if (globalThis.Deno?.readTextFile) {
      const content = await globalThis.Deno.readTextFile(path);
      if (encoding === 'utf-8' || encoding === 'utf8') {
        return content;
      }
      return new TextEncoder().encode(content);
    }
    return new Uint8Array(0);
  } catch (error) {
    throw new Error(`ENOENT: no such file or directory, open '${path}'`);
  }
};

const existsSync = (path) => {
  try {
    if (globalThis.Deno?.statSync) {
      globalThis.Deno.statSync(path);
      return true;
    }
    return false;
  } catch (error) {
    return false;
  }
};

const statSync = (path) => {
  try {
    if (globalThis.Deno?.statSync) {
      const stat = globalThis.Deno.statSync(path);
      return {
        isFile: () => stat.isFile,
        isDirectory: () => stat.isDirectory,
        isSymbolicLink: () => stat.isSymlink,
        size: stat.size,
        mtime: stat.mtime,
        atime: stat.atime,
        birthtime: stat.birthtime,
        mode: stat.mode || 0,
        uid: stat.uid || 0,
        gid: stat.gid || 0,
      };
    }
    throw new Error(`ENOENT: no such file or directory, stat '${path}'`);
  } catch (error) {
    throw new Error(`ENOENT: no such file or directory, stat '${path}'`);
  }
};

const lstatSync = statSync;

const realpathSync = (path) => {
  try {
    if (globalThis.Deno?.realPathSync) {
      return globalThis.Deno.realPathSync(path);
    }
    return path;
  } catch (error) {
    return path;
  }
};

const constants = {
  F_OK: 0,
  R_OK: 4,
  W_OK: 2,
  X_OK: 1,
};

const readdirSync = (path) => {
  try {
    if (globalThis.Deno?.readDirSync) {
      const entries = [];
      for (const entry of globalThis.Deno.readDirSync(path)) {
        entries.push(entry.name);
      }
      return entries;
    }
    return [];
  } catch (error) {
    throw new Error(`ENOENT: no such file or directory, scandir '${path}'`);
  }
};

export default {
  readFileSync,
  readFile,
  existsSync,
  readdirSync,
  statSync,
  lstatSync,
  realpathSync,
  constants,
  writeFileSync: () => {},
  writeFile: () => Promise.resolve(),
  exists: () => Promise.resolve(false),
  mkdirSync: () => {},
  mkdir: () => Promise.resolve(),
  readdir: () => Promise.resolve([]),
};

const promises = {
  readFile,
  writeFile: () => Promise.resolve(),
  readdir: () => Promise.resolve([]),
  stat: async (path) => {
    try {
      if (globalThis.Deno?.stat) {
        const stat = await globalThis.Deno.stat(path);
        return {
          isFile: () => stat.isFile,
          isDirectory: () => stat.isDirectory,
          isSymbolicLink: () => stat.isSymlink,
          size: stat.size,
          mtime: stat.mtime,
          atime: stat.atime,
          birthtime: stat.birthtime,
        };
      }
      throw new Error(`ENOENT: no such file or directory, stat '${path}'`);
    } catch (error) {
      throw new Error(`ENOENT: no such file or directory, stat '${path}'`);
    }
  },
  mkdir: () => Promise.resolve(),
  rm: () => Promise.resolve(),
  rmdir: () => Promise.resolve(),
  unlink: () => Promise.resolve(),
  access: () => Promise.resolve(),
};

const stat = async (path, callback) => {
  try {
    if (globalThis.Deno?.stat) {
      const statResult = await globalThis.Deno.stat(path);
      const result = {
        isFile: () => statResult.isFile,
        isDirectory: () => statResult.isDirectory,
        isSymbolicLink: () => statResult.isSymlink,
        size: statResult.size,
        mtime: statResult.mtime,
        atime: statResult.atime,
        birthtime: statResult.birthtime,
      };
      if (callback) callback(null, result);
      return result;
    }
    const error = new Error(`ENOENT: no such file or directory, stat '${path}'`);
    if (callback) callback(error);
    throw error;
  } catch (error) {
    if (callback) callback(error);
    throw error;
  }
};

const watchFile = (path, options, listener) => {
  if (typeof options === 'function') {
    listener = options;
  }
  console.warn('fs.watchFile is not supported in this environment');
  return { close: () => {} };
};

const unwatchFile = (path, listener) => {
  console.warn('fs.unwatchFile is not supported in this environment');
};

const watch = (path, options, listener) => {
  if (typeof options === 'function') {
    listener = options;
  }
  console.warn('fs.watch is not supported in this environment');
  return { close: () => {} };
};

export { readFileSync, readFile, existsSync, readdirSync, statSync, lstatSync, realpathSync, constants, promises, stat, watchFile, unwatchFile, watch };
export const writeFileSync = () => {};
export const writeFile = () => Promise.resolve();
export const exists = () => Promise.resolve(false);
export const mkdirSync = () => {};
export const mkdir = () => Promise.resolve();
export const readdir = () => Promise.resolve([]);
export const __esModule = true;
"#;

pub const JSX_RUNTIME_STUB: &str = r#"
export function jsx(type, props, key) {
  const element = {
    $$typeof: Symbol.for('react.transitional.element'),
    type,
    props: props || {},
    key: key || null
  };

  if (props && props.children !== undefined) {
    element.props = { ...element.props, children: props.children };
  }

  return element;
}

export function jsxs(type, props, key) {
  return jsx(type, props, key);
}

export function Fragment(props) {
  return props?.children || null;
}

export default { jsx, jsxs, Fragment };
"#;

pub const LOADER_STUB_TEMPLATE: &str = r#"
// Auto-generated loader stub for {component_id}

if (typeof globalThis.registerModule === 'function') {{
    globalThis.registerModule({{}}, '{component_id}');
}}

if (typeof globalThis['~rsc'].functions === 'undefined') {{
    globalThis['~rsc'].functions = {{}};
}}

if (typeof globalThis.'~rsc'].modules == 'undefined') {{
    globalThis['~rsc'].modules = {{}};
}}

globalThis['~rsc'].modules['{component_id}'] = {{
    __isLoaderStub: true,
    __awaitingRegistration: true
}};

export default {{
    __isLoaderStub: true,
    __componentId: "{component_id}",
    __timestamp: Date.now()
}};
"#;

pub const FALLBACK_MODULE_TEMPLATE: &str = r#"
// Dynamic fallback module for: {module_name}

if (typeof globalThis['~rsc'].modules === 'undefined') {{
    globalThis['~rsc'].modules = {{}};
}}

globalThis['~rsc'].modules['{module_name}'] = {{
    __isFallback: true,
    __timestamp: Date.now()
}};

export default {{
    __isFallback: true,
    __module: "{module_name}",
    __timestamp: Date.now()
}};
"#;

pub const NODE_PATH_STUB: &str = r#"
// ESM-compatible stub for node:path

export function join(...parts) {
  if (parts.length === 0) return '.';

  const joined = parts
    .filter(part => part && part.length > 0)
    .join('/')
    .replace(/\/+/g, '/');

  return joined || '.';
}

export function dirname(path) {
  if (!path || path === '/') return '/';
  const normalized = path.replace(/\/+$/, '');
  const lastSlash = normalized.lastIndexOf('/');
  if (lastSlash === -1) return '.';
  if (lastSlash === 0) return '/';
  return normalized.slice(0, lastSlash);
}

export function basename(path, ext) {
  if (!path) return '';
  const base = path.split('/').filter(Boolean).pop() || '';
  if (ext && base.endsWith(ext)) {
    return base.slice(0, -ext.length);
  }
  return base;
}

export function extname(path) {
  if (!path) return '';
  const base = basename(path);
  const lastDot = base.lastIndexOf('.');
  if (lastDot === -1 || lastDot === 0) return '';
  return base.slice(lastDot);
}

export function resolve(...paths) {
  let resolvedPath = '';
  let resolvedAbsolute = false;

  const cwd = (() => {
    try {
      if (globalThis.Deno?.cwd) {
        return globalThis.Deno.cwd();
      }
      if (globalThis.process?.cwd) {
        return globalThis.process.cwd();
      }
      return '/';
    } catch {
      return '/';
    }
  })();

  for (let i = paths.length - 1; i >= -1 && !resolvedAbsolute; i--) {
    const path = i >= 0 ? paths[i] : cwd;

    if (!path || path.length === 0) {
      continue;
    }

    resolvedPath = path + '/' + resolvedPath;
    resolvedAbsolute = path.charAt(0) === '/';
  }

  resolvedPath = normalizeArray(
    resolvedPath.split('/').filter(p => p.length > 0),
    !resolvedAbsolute
  ).join('/');

  return (resolvedAbsolute ? '/' : '') + resolvedPath || '.';
}

function normalizeArray(parts, allowAboveRoot) {
  const res = [];
  for (let i = 0; i < parts.length; i++) {
    const p = parts[i];
    if (!p || p === '.') continue;
    if (p === '..') {
      if (res.length && res[res.length - 1] !== '..') {
        res.pop();
      } else if (allowAboveRoot) {
        res.push('..');
      }
    } else {
      res.push(p);
    }
  }
  return res;
}

export function isAbsolute(path) {
  return path && path.length > 0 && path.charAt(0) === '/';
}

export function normalize(path) {
  if (!path || path.length === 0) return '.';

  const isAbs = isAbsolute(path);
  const trailingSlash = path.charAt(path.length - 1) === '/';

  const normalized = normalizeArray(
    path.split('/').filter(p => p.length > 0),
    !isAbs
  ).join('/');

  if (!normalized && !isAbs) return '.';
  if (normalized && trailingSlash) return normalized + '/';

  return (isAbs ? '/' : '') + normalized;
}

export function relative(from, to) {
  from = resolve(from);
  to = resolve(to);

  if (from === to) return '';

  const fromParts = from.split('/').filter(Boolean);
  const toParts = to.split('/').filter(Boolean);

  let i = 0;
  while (i < fromParts.length && i < toParts.length && fromParts[i] === toParts[i]) {
    i++;
  }

  const upCount = fromParts.length - i;
  const relativeParts = [];

  for (let j = 0; j < upCount; j++) {
    relativeParts.push('..');
  }

  return relativeParts.concat(toParts.slice(i)).join('/') || '.';
}

export const sep = '/';
export const delimiter = ':';
export const posix = {
  join,
  dirname,
  basename,
  extname,
  resolve,
  isAbsolute,
  normalize,
  relative,
  sep,
  delimiter
};

export default {
  join,
  dirname,
  basename,
  extname,
  resolve,
  isAbsolute,
  normalize,
  relative,
  sep,
  delimiter,
  posix
};

export const __esModule = true;
"#;

pub const NODE_PROCESS_STUB: &str = r#"
// ESM-compatible bridge for node:process to Deno APIs

const cwd = () => {
  try {
    if (globalThis.Deno?.cwd) {
      return globalThis.Deno.cwd();
    }
    return '/';
  } catch (error) {
    return '/';
  }
};

const env = new Proxy({}, {
  get(target, prop) {
    try {
      if (globalThis.process?.env && prop in globalThis.process.env) {
        return globalThis.process.env[prop];
      }
      if (globalThis.Deno?.env?.get) {
        return globalThis.Deno.env.get(prop);
      }
      return undefined;
    } catch (error) {
      return undefined;
    }
  },
  has(target, prop) {
    try {
      if (globalThis.process?.env && prop in globalThis.process.env) {
        return true;
      }
      if (globalThis.Deno?.env?.get) {
        return globalThis.Deno.env.get(prop) !== undefined;
      }
      return false;
    } catch (error) {
      return false;
    }
  },
  ownKeys(target) {
    try {
      if (globalThis.process?.env) {
        return Object.keys(globalThis.process.env);
      }
      return [];
    } catch (error) {
      return [];
    }
  },
  getOwnPropertyDescriptor(target, prop) {
    try {
      if (globalThis.process?.env && prop in globalThis.process.env) {
        return {
          enumerable: true,
          configurable: true,
          value: globalThis.process.env[prop]
        };
      }
      return undefined;
    } catch (error) {
      return undefined;
    }
  }
});

const argv = ['node'];

const platform = (() => {
  try {
    if (globalThis.Deno?.build?.os) {
      const os = globalThis.Deno.build.os;
      if (os === 'darwin') return 'darwin';
      if (os === 'linux') return 'linux';
      if (os === 'windows') return 'win32';
    }
    return 'linux';
  } catch (error) {
    return 'linux';
  }
})();

const arch = (() => {
  try {
    if (globalThis.Deno?.build?.arch) {
      const a = globalThis.Deno.build.arch;
      if (a === 'x86_64') return 'x64';
      if (a === 'aarch64') return 'arm64';
      return a;
    }
    return 'x64';
  } catch (error) {
    return 'x64';
  }
})();

const version = 'v20.0.0';
const versions = {
  node: '20.0.0',
  v8: '11.0.0',
};

const pid = (() => {
  try {
    if (globalThis.Deno?.pid) {
      return globalThis.Deno.pid;
    }
    return 1;
  } catch (error) {
    return 1;
  }
})();

const ppid = 0;
const execPath = '/usr/bin/node';
const execArgv = [];

const memoryUsage = () => ({
  rss: 0,
  heapTotal: 0,
  heapUsed: 0,
  external: 0,
  arrayBuffers: 0,
});

const uptime = () => 0;

export default {
  cwd,
  env,
  argv,
  platform,
  arch,
  version,
  versions,
  pid,
  ppid,
  execPath,
  execArgv,
  memoryUsage,
  uptime,
  nextTick: (fn) => setTimeout(fn, 0),
  exit: (code = 0) => {
    if (globalThis.Deno?.exit) {
      globalThis.Deno.exit(code);
    }
  },
};

export { cwd, env, argv, platform, arch, version, versions, pid, ppid, execPath, execArgv, memoryUsage, uptime };
export const nextTick = (fn) => setTimeout(fn, 0);
export const exit = (code = 0) => {
  if (globalThis.Deno?.exit) {
    globalThis.Deno.exit(code);
  }
};
export const __esModule = true;
"#;

pub const NODE_URL_STUB: &str = r#"
// ESM-compatible stub for node:url

export function fileURLToPath(url) {
  if (typeof url === 'string') {
    if (url.startsWith('file://')) {
      let path = url.slice(7);

      if (path.match(/^\/[a-zA-Z]:\//)) {
        path = path.slice(1);
      }

      try {
        path = decodeURIComponent(path);
      } catch (e) {
        // If decoding fails, use the path as-is
      }

      return path;
    }
    return url;
  }

  if (url && typeof url === 'object' && url.protocol === 'file:') {
    return fileURLToPath(url.href || url.toString());
  }

  return String(url);
}

export function pathToFileURL(path) {
  if (typeof path !== 'string') {
    path = String(path);
  }

  if (path.startsWith('/')) {
    return new URL('file://' + path);
  }

  if (path.match(/^[a-zA-Z]:\\/)) {
    return new URL('file:///' + path.replace(/\\/g, '/'));
  }

  const cwd = (() => {
    try {
      if (globalThis.Deno?.cwd) {
        return globalThis.Deno.cwd();
      }
      if (globalThis.process?.cwd) {
        return globalThis.process.cwd();
      }
      return '/';
    } catch {
      return '/';
    }
  })();

  const fullPath = cwd + '/' + path;
  return new URL('file://' + fullPath);
}

export function format(urlObject) {
  if (typeof urlObject === 'string') {
    return urlObject;
  }

  if (!urlObject || typeof urlObject !== 'object') {
    return '';
  }

  const protocol = urlObject.protocol || '';
  const hostname = urlObject.hostname || urlObject.host || '';
  const port = urlObject.port ? ':' + urlObject.port : '';
  const pathname = urlObject.pathname || '';
  const search = urlObject.search || '';
  const hash = urlObject.hash || '';

  return protocol + '//' + hostname + port + pathname + search + hash;
}

export function parse(urlString, parseQueryString = false) {
  try {
    const url = new URL(urlString);
    return {
      protocol: url.protocol,
      hostname: url.hostname,
      port: url.port,
      pathname: url.pathname,
      search: url.search,
      hash: url.hash,
      host: url.host,
      href: url.href,
    };
  } catch (e) {
    return {
      protocol: null,
      hostname: null,
      port: null,
      pathname: urlString,
      search: null,
      hash: null,
      host: null,
      href: urlString,
    };
  }
}

export default {
  fileURLToPath,
  pathToFileURL,
  format,
  parse,
};

export const __esModule = true;
"#;

pub const REACT_STUB: &str = r#"
// React stub for Deno environment

const createElement = (type, props, ...children) => {
  if (typeof type === 'string') {
    // HTML element
    return { type, props: props || {}, children: children.flat() };
  }
  // Component
  return { type, props: props || {}, children: children.flat() };
};

const Fragment = Symbol.for('react.fragment');
const Suspense = (props) => props.children;
const useState = (initial) => [initial, () => {}];
const useEffect = () => {};
const useContext = () => null;
const use = (promise) => {
  if (promise && typeof promise.then === 'function') {
    throw promise; // Suspense behavior
  }
  return promise;
};
const createContext = (defaultValue) => ({
  Provider: ({ children }) => children,
  Consumer: ({ children }) => children(defaultValue),
  _currentValue: defaultValue
});
const memo = (component) => component;
const useRef = (initial) => ({ current: initial });
const useCallback = (fn) => fn;
const useMemo = (fn) => fn();
const createRef = () => ({ current: null });
const lazy = (factory) => factory;
const StrictMode = ({ children }) => children;
const useTransition = () => [false, (fn) => fn()];
const useDeferredValue = (value) => value;
const useId = () => Math.random().toString(36);
const startTransition = (fn) => fn();
const flushSync = (fn) => fn();
const unstable_act = (fn) => fn();

export {
  createElement,
  Fragment,
  Suspense,
  useState,
  useEffect,
  useContext,
  use,
  createContext,
  memo,
  useRef,
  useCallback,
  useMemo,
  createRef,
  lazy,
  StrictMode,
  useTransition,
  useDeferredValue,
  useId,
  startTransition,
  flushSync,
  unstable_act
};

export default {
  createElement,
  Fragment,
  Suspense,
  useState,
  useEffect,
  useContext,
  use,
  createContext,
  memo,
  useRef,
  useCallback,
  useMemo,
  createRef,
  lazy,
  StrictMode,
  useTransition,
  useDeferredValue,
  useId,
  startTransition,
  flushSync,
  unstable_act
};
"#;

pub fn create_generic_module_stub(module_path: &str) -> String {
    format!(
        r#"
// Generic fallback stub for node module: {module_path}

export default {{
  name: '{module_path}',
  isStub: true
}};

export const useState = (initialState) => [initialState, () => {{}}];
export const useEffect = (fn, deps) => {{}};
export const createElement = (type, props, ...children) => ({{ type, props, children }});
export const render = () => {{}};
export const Fragment = Symbol('fragment');
"#
    )
}

pub fn create_component_stub(component_name: &str) -> String {
    format!(
        r#"
// Auto-generated stub for component: {component_name}

const moduleExports = {{
    __isStub: true,
    __componentName: "{component_name}",
    __awaitingRegistration: true
}};

export function ~rari_register() {{
    if (typeof globalThis.registerModule === 'function') {{
        globalThis.registerModule(moduleExports, '{component_name}');
    }}

    if (typeof globalThis['~rsc'].functions === 'undefined') {{
        globalThis['~rsc'].functions = {{}};
    }}

    if (typeof globalThis['~rsc'].modules === 'undefined') {{
        globalThis['~rsc'].modules = {{}};
    }}

    globalThis['~rsc'].modules['{component_name}'] = moduleExports;
}}

export default moduleExports;
"#
    )
}

pub const NODE_STREAM_STUB: &str = r#"
// ESM-compatible stub for node:stream

class EventEmitter {
  constructor() {
    this._events = {};
  }

  on(event, listener) {
    if (!this._events[event]) {
      this._events[event] = [];
    }
    this._events[event].push(listener);
    return this;
  }

  emit(event, ...args) {
    if (!this._events[event]) return false;
    this._events[event].forEach(listener => {
      try {
        listener.apply(this, args);
      } catch (error) {
        console.error('EventEmitter error:', error);
      }
    });
    return true;
  }

  removeListener(event, listener) {
    if (!this._events[event]) return this;
    this._events[event] = this._events[event].filter(l => l !== listener);
    return this;
  }

  off(event, listener) {
    return this.removeListener(event, listener);
  }
}

export class Readable extends EventEmitter {
  constructor(options = {}) {
    super();
    this.readable = true;
  }

  pipe(destination) {
    this.on('data', (chunk) => {
      if (destination.write) destination.write(chunk);
    });
    this.on('end', () => {
      if (destination.end) destination.end();
    });
    return destination;
  }

  push(chunk) {
    if (chunk === null) {
      this.emit('end');
      return false;
    }
    this.emit('data', chunk);
    return true;
  }
}

export class Writable extends EventEmitter {
  constructor(options = {}) {
    super();
    this.writable = true;
  }

  write(chunk, encoding, callback) {
    if (typeof encoding === 'function') {
      callback = encoding;
    }
    if (callback) callback();
    return true;
  }

  end(chunk, encoding, callback) {
    if (chunk) this.write(chunk, encoding);
    this.emit('finish');
    if (callback) callback();
    return this;
  }
}

export class Transform extends Readable {
  constructor(options = {}) {
    super(options);
    this.writable = true;
  }

  write(chunk, encoding, callback) {
    this.push(chunk);
    if (callback) callback();
    return true;
  }

  end(chunk, encoding, callback) {
    if (chunk) this.write(chunk, encoding);
    this.push(null);
    if (callback) callback();
    return this;
  }
}

export class PassThrough extends Transform {}

export class Duplex extends Readable {
  constructor(options = {}) {
    super(options);
    this.writable = true;
  }

  write(chunk, encoding, callback) {
    if (callback) callback();
    return true;
  }

  end(chunk, encoding, callback) {
    if (chunk) this.write(chunk, encoding);
    this.push(null);
    if (callback) callback();
    return this;
  }
}

export const Stream = Readable;

export default {
  Readable,
  Writable,
  Transform,
  PassThrough,
  Duplex,
  Stream,
};

export const __esModule = true;
"#;

pub const NODE_BUFFER_STUB: &str = r#"
// ESM-compatible stub for node:buffer

class BufferImpl extends Uint8Array {
  toString(encoding = 'utf8') {
    if (encoding === 'utf8' || encoding === 'utf-8') {
      return new TextDecoder().decode(this);
    }
    if (encoding === 'hex') {
      return Array.from(this).map(b => b.toString(16).padStart(2, '0')).join('');
    }
    if (encoding === 'base64') {
      return btoa(String.fromCharCode(...this));
    }
    return new TextDecoder().decode(this);
  }

  toJSON() {
    return { type: 'Buffer', data: Array.from(this) };
  }
}

BufferImpl.from = function(arg, encoding) {
  if (typeof arg === 'string') {
    return new BufferImpl(new TextEncoder().encode(arg));
  }
  return new BufferImpl(arg);
};

BufferImpl.alloc = function(size) {
  return new BufferImpl(size);
};

BufferImpl.isBuffer = function(obj) {
  return obj instanceof BufferImpl || obj instanceof Uint8Array;
};

export const Buffer = BufferImpl;
export default { Buffer };
export const __esModule = true;
"#;

pub const NODE_OS_STUB: &str = r#"
// ESM-compatible stub for node:os

export function platform() {
  try {
    const os = globalThis.Deno?.build?.os;
    if (os === 'darwin') return 'darwin';
    if (os === 'linux') return 'linux';
    if (os === 'windows') return 'win32';
    return 'linux';
  } catch {
    return 'linux';
  }
}

export function arch() {
  try {
    const a = globalThis.Deno?.build?.arch;
    if (a === 'x86_64') return 'x64';
    if (a === 'aarch64') return 'arm64';
    return 'x64';
  } catch {
    return 'x64';
  }
}

export function type() {
  try {
    const os = globalThis.Deno?.build?.os;
    if (os === 'darwin') return 'Darwin';
    if (os === 'linux') return 'Linux';
    if (os === 'windows') return 'Windows_NT';
    return 'Linux';
  } catch {
    return 'Linux';
  }
}

export function homedir() {
  try {
    return globalThis.Deno?.env?.get('HOME') || globalThis.Deno?.env?.get('USERPROFILE') || '/';
  } catch {
    return '/';
  }
}

export function tmpdir() {
  try {
    return globalThis.Deno?.env?.get('TMPDIR') || globalThis.Deno?.env?.get('TMP') || '/tmp';
  } catch {
    return '/tmp';
  }
}

export const EOL = '\n';

export default { platform, arch, type, homedir, tmpdir, EOL };
export const __esModule = true;
"#;

pub const NODE_EVENTS_STUB: &str = r#"
// ESM-compatible stub for node:events

export class EventEmitter {
  constructor() {
    this._events = {};
  }

  on(event, listener) {
    if (!this._events[event]) {
      this._events[event] = [];
    }
    this._events[event].push(listener);
    return this;
  }

  emit(event, ...args) {
    if (!this._events[event]) return false;
    this._events[event].forEach(listener => {
      try {
        listener.apply(this, args);
      } catch (error) {
        console.error('EventEmitter error:', error);
      }
    });
    return true;
  }

  removeListener(event, listener) {
    if (!this._events[event]) return this;
    this._events[event] = this._events[event].filter(l => l !== listener);
    return this;
  }

  off(event, listener) {
    return this.removeListener(event, listener);
  }

  removeAllListeners(event) {
    if (event) {
      delete this._events[event];
    } else {
      this._events = {};
    }
    return this;
  }
}

export default EventEmitter;
export const __esModule = true;
"#;

pub const NODE_CHILD_PROCESS_STUB: &str = r#"
// ESM-compatible stub for node:child_process

export function spawn(command, args = [], options = {}) {
  console.warn('child_process.spawn is not supported in this environment');
  return {
    stdout: { on: () => {}, pipe: () => {} },
    stderr: { on: () => {}, pipe: () => {} },
    stdin: { write: () => {}, end: () => {} },
    on: () => {},
    kill: () => {},
    pid: -1,
  };
}

export function exec(command, options, callback) {
  if (typeof options === 'function') {
    callback = options;
    options = {};
  }
  console.warn('child_process.exec is not supported in this environment');
  if (callback) {
    setTimeout(() => callback(new Error('child_process not supported'), '', ''), 0);
  }
  return spawn(command);
}

export function execFile(file, args, options, callback) {
  if (typeof args === 'function') {
    callback = args;
    args = [];
    options = {};
  } else if (typeof options === 'function') {
    callback = options;
    options = {};
  }
  console.warn('child_process.execFile is not supported in this environment');
  if (callback) {
    setTimeout(() => callback(new Error('child_process not supported'), '', ''), 0);
  }
  return spawn(file, args, options);
}

export function fork(modulePath, args = [], options = {}) {
  console.warn('child_process.fork is not supported in this environment');
  return spawn(modulePath, args, options);
}

export function execSync(command, options = {}) {
  const parts = command.match(/(?:[^\s"]+|"[^"]*")+/g) || [];
  const cmd = parts[0];
  const args = parts.slice(1).map(arg => arg.replace(/^"|"$/g, ''));

  try {
    const denoCmd = new Deno.Command(cmd, {
      args,
      cwd: options.cwd,
      stdout: 'piped',
      stderr: 'piped',
    });

    const { stdout, stderr, success } = denoCmd.outputSync();

    if (!success && options.stdio !== 'ignore') {
      const error = new Error(`Command failed: ${command}`);
      error.stderr = new TextDecoder().decode(stderr);
      throw error;
    }

    const output = new TextDecoder().decode(stdout);
    return options.encoding === 'utf-8' || options.encoding === 'utf8' ? output : stdout;
  } catch (error) {
    console.error('execSync error:', error);
    throw error;
  }
}

export function execFileSync(file, args = [], options = {}) {
  console.warn('child_process.execFileSync is not supported in this environment');
  return '';
}

export function spawnSync(command, args = [], options = {}) {
  console.warn('child_process.spawnSync is not supported in this environment');
  return {
    pid: -1,
    output: [],
    stdout: '',
    stderr: '',
    status: 1,
    signal: null,
    error: new Error('child_process not supported'),
  };
}

export default {
  spawn,
  exec,
  execFile,
  fork,
  execSync,
  execFileSync,
  spawnSync,
};

export const __esModule = true;
"#;

pub const NODE_FS_PROMISES_STUB: &str = r#"
// ESM-compatible stub for node:fs/promises

export async function readFile(path, encoding) {
  try {
    if (globalThis.Deno?.readTextFile) {
      const content = await globalThis.Deno.readTextFile(path);
      if (encoding === 'utf-8' || encoding === 'utf8') {
        return content;
      }
      return new TextEncoder().encode(content);
    }
    return new Uint8Array(0);
  } catch (error) {
    throw new Error(`ENOENT: no such file or directory, open '${path}'`);
  }
}

export async function writeFile(path, data, encoding) {
  try {
    if (globalThis.Deno?.writeTextFile) {
      const content = typeof data === 'string' ? data : new TextDecoder().decode(data);
      await globalThis.Deno.writeTextFile(path, content);
    }
  } catch (error) {
    throw new Error(`ENOENT: no such file or directory, open '${path}'`);
  }
}

export async function stat(path) {
  try {
    if (globalThis.Deno?.stat) {
      const stat = await globalThis.Deno.stat(path);
      return {
        isFile: () => stat.isFile,
        isDirectory: () => stat.isDirectory,
        isSymbolicLink: () => stat.isSymlink,
        size: stat.size,
        mtime: stat.mtime,
        atime: stat.atime,
        birthtime: stat.birthtime,
      };
    }
    throw new Error(`ENOENT: no such file or directory, stat '${path}'`);
  } catch (error) {
    throw new Error(`ENOENT: no such file or directory, stat '${path}'`);
  }
}

export async function readdir(path) {
  try {
    if (globalThis.Deno?.readDir) {
      const entries = [];
      for await (const entry of globalThis.Deno.readDir(path)) {
        entries.push(entry.name);
      }
      return entries;
    }
    return [];
  } catch (error) {
    throw new Error(`ENOENT: no such file or directory, scandir '${path}'`);
  }
}

export async function mkdir(path, options) {
  return Promise.resolve();
}

export async function rm(path, options) {
  return Promise.resolve();
}

export async function rmdir(path, options) {
  return Promise.resolve();
}

export async function unlink(path) {
  return Promise.resolve();
}

export async function access(path, mode) {
  return Promise.resolve();
}

export default {
  readFile,
  writeFile,
  stat,
  readdir,
  mkdir,
  rm,
  rmdir,
  unlink,
  access,
};

export const __esModule = true;
"#;

pub const NODE_MODULE_STUB: &str = r#"
// ESM-compatible stub for node:module

export function createRequire(filename) {
  return function require(id) {
    console.warn(`require('${id}') is not supported in this environment`);
    return {};
  };
}

export function isBuiltin(moduleName) {
  const builtins = [
    'assert', 'buffer', 'child_process', 'cluster', 'crypto',
    'dgram', 'dns', 'domain', 'events', 'fs', 'http', 'https',
    'net', 'os', 'path', 'punycode', 'querystring', 'readline',
    'stream', 'string_decoder', 'timers', 'tls', 'tty', 'url',
    'util', 'v8', 'vm', 'zlib'
  ];
  const name = moduleName.replace(/^node:/, '');
  return builtins.includes(name);
}

export const builtinModules = [
  'assert', 'buffer', 'child_process', 'cluster', 'crypto',
  'dgram', 'dns', 'domain', 'events', 'fs', 'http', 'https',
  'net', 'os', 'path', 'punycode', 'querystring', 'readline',
  'stream', 'string_decoder', 'timers', 'tls', 'tty', 'url',
  'util', 'v8', 'vm', 'zlib'
];

export default {
  createRequire,
  isBuiltin,
  builtinModules,
};

export const __esModule = true;
"#;
