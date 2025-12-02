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
  writeFileSync: () => {},
  writeFile: () => Promise.resolve(),
  exists: () => Promise.resolve(false),
  mkdirSync: () => {},
  mkdir: () => Promise.resolve(),
  readdir: () => Promise.resolve([]),
};

export { readFileSync, readFile, existsSync, readdirSync };
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
    $typeof: Symbol.for('react.element'),
    type,
    props: props || {},
    key: key || null,
    ref: null
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

if (typeof globalThis.__rsc_functions === 'undefined') {{
    globalThis.__rsc_functions = {{}};
}}

if (typeof globalThis.__rsc_modul== 'undefined') {{
    globalThis.__rsc_modules = {{}};
}}

globalThis.__rsc_modules['{component_id}'] = {{
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

if (typeof globalThis.__rsc_modules === 'undefined') {{
    globalThis.__rsc_modules = {{}};
}}

globalThis.__rsc_modules['{module_name}'] = {{
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
  return parts.join('/');
}
export function dirname(path) {
  return path.split('/').slice(0, -1).join('/');
}
export function basename(path) {
  return path.split('/').pop();
}
export function extname(path) {
  const parts = path.split('.');
  return parts.length > 1 ? `.${parts.pop()}` : '';
}
export function resolve(...parts) {
  return '/' + parts.join('/');
}
export function isAbsolute(path) {
  return path.startsWith('/');
}
export default {
  join,
  dirname,
  basename,
  extname,
  resolve,
  isAbsolute,
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

export default {
  cwd,
  env,
  argv,
  platform,
  nextTick: (fn) => setTimeout(fn, 0),
  exit: (code = 0) => {
    if (globalThis.Deno?.exit) {
      globalThis.Deno.exit(code);
    }
  },
};

export { cwd, env, argv, platform };
export const nextTick = (fn) => setTimeout(fn, 0);
export const exit = (code = 0) => {
  if (globalThis.Deno?.exit) {
    globalThis.Deno.exit(code);
  }
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

export function __rari_register() {{
    if (typeof globalThis.registerModule === 'function') {{
        globalThis.registerModule(moduleExports, '{component_name}');
    }}

    if (typeof globalThis.__rsc_functions === 'undefined') {{
        globalThis.__rsc_functions = {{}};
    }}

    if (typeof globalThis.__rsc_modules === 'undefined') {{
        globalThis.__rsc_modules = {{}};
    }}

    globalThis.__rsc_modules['{component_name}'] = moduleExports;
}}

export default moduleExports;
"#
    )
}
