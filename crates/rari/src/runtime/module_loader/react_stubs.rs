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

pub const REACT_STUB: &str = r#"
const createElement = (type, props, ...children) => {
  if (typeof type === 'string') {
    return { type, props: props || {}, children: children.flat() };
  }

  return { type, props: props || {}, children: children.flat() };
};

const Fragment = Symbol.for('react.fragment');
const Suspense = (props) => props.children;
const useState = (initial) => [initial, () => {}];
const useEffect = () => {};
const useContext = () => null;
const use = (promise) => {
  if (promise && typeof promise.then === 'function') {
    throw promise;
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

const cache = (fn) => {
  const hasOps = typeof Deno?.core?.ops?.op_cache_get === 'function'
    && typeof Deno?.core?.ops?.op_cache_set === 'function';

  if (!hasOps) {
    return fn;
  }

  const ops = Deno.core.ops;

  function generateCacheKey(fn, args) {
    const fnName = fn.name || 'anonymous';
    const argsKey = JSON.stringify(args, (_, value) => {
      if (typeof value === 'function') return '[Function]';
      if (value instanceof Error) return `[Error: ${value.message}]`;
      if (value instanceof Date) return value.toISOString();
      if (value instanceof RegExp) return value.toString();
      if (typeof value === 'symbol') return value.toString();
      if (typeof value === 'bigint') return value.toString();
      return value;
    });

    return `${fnName}:${argsKey}`;
  }

  return async function cachedFunction(...args) {
    const cacheKey = generateCacheKey(fn, args);

    const cached = ops.op_cache_get(cacheKey);
    if (cached !== null && cached !== undefined) {
      return cached;
    }

    const result = await fn(...args);

    ops.op_cache_set(cacheKey, result);

    return result;
  };
};

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
  unstable_act,
  cache
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
  unstable_act,
  cache
};
"#;
