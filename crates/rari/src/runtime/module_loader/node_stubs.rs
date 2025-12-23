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

export { readFileSync, readFile, existsSync, readdirSync, statSync, lstatSync, realpathSync, constants };
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

const Fragment = Symbol('react.fragment');
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
const forwardRef = (component) => component;
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
  forwardRef,
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
  forwardRef,
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
