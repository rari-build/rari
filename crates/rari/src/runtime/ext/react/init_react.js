globalThis.createElement = function (type, props, ...children) {
  const element = {
    type,
    props: {
      ...props,
      children: children.length === 1 ? children[0] : children,
    },
    key: props?.key || null,
    ref: props?.ref || null,
    $$typeof: Symbol.for('react.element'),
  }
  return element
}

globalThis.Fragment = Symbol.for('react.fragment')

globalThis.Suspense = function Suspense({ children, fallback }) {
  return globalThis.createElement('suspense', { fallback }, children)
}

globalThis.createContext = function (defaultValue) {
  const context = {
    $$typeof: Symbol.for('react.context'),
    _currentValue: defaultValue,
    _currentValue2: defaultValue,
    Provider: function Provider({ value, children }) {
      context._currentValue = value
      return children
    },
    Consumer: function Consumer({ children }) {
      return children(context._currentValue)
    },
  }
  return context
}

globalThis.useContext = function (context) {
  return context._currentValue
}

globalThis.memo = function memo(Component, areEqual) {
  const MemoComponent = function (props) {
    return Component(props)
  }
  MemoComponent.$$typeof = Symbol.for('react.memo')
  MemoComponent.type = Component
  MemoComponent.compare = areEqual || null
  return MemoComponent
}

globalThis.forwardRef = function forwardRef(render) {
  const ForwardRef = function (props) {
    return render(props, null)
  }
  ForwardRef.$$typeof = Symbol.for('react.forward_ref')
  ForwardRef.render = render
  return ForwardRef
}

globalThis.createRef = function () {
  return { current: null }
}

globalThis.use = function use(resource) {
  if (resource && typeof resource.then === 'function') {
    throw resource
  }
  if (resource && resource.$$typeof === Symbol.for('react.context')) {
    return globalThis.use(resource)
  }
  throw new Error('use() can only be called with promises or context objects')
}

globalThis.lazy = function lazy(_loadComponent) {
  return function LazyComponent(_props) {
    throw new Error('Lazy components require client-side rendering in this RSC framework')
  }
}

globalThis.StrictMode = function StrictMode({ children }) {
  return children
}

function createClientOnlyHook(hookName) {
  return function () {
    throw new Error(
      `${hookName} is a client-side only hook. `
      + 'Use "use client" directive at the top of your component file to run this code on the client.',
    )
  }
}

globalThis.useState = createClientOnlyHook('useState')
globalThis.useEffect = createClientOnlyHook('useEffect')
globalThis.useRef = createClientOnlyHook('useRef')
globalThis.useCallback = createClientOnlyHook('useCallback')
globalThis.useMemo = createClientOnlyHook('useMemo')
globalThis.useTransition = createClientOnlyHook('useTransition')
globalThis.useDeferredValue = createClientOnlyHook('useDeferredValue')
globalThis.useId = function () {
  return `:r${Math.random().toString(36).substr(2, 9)}:`
}

globalThis.startTransition = createClientOnlyHook('startTransition')
globalThis.flushSync = createClientOnlyHook('flushSync')
globalThis.unstable_act = createClientOnlyHook('unstable_act')

globalThis.ReactDOMServer = {
  renderToString(element) {
    return renderElementToString(element)
  },
  renderToStaticMarkup(element) {
    return renderElementToString(element, true)
  },
}

function renderElementToString(element, isStatic = false) {
  if (element === null || element === undefined || typeof element === 'boolean') {
    return ''
  }

  if (typeof element === 'string' || typeof element === 'number') {
    return escapeHtml(String(element))
  }

  if (Array.isArray(element)) {
    return element.map(child => renderElementToString(child, isStatic)).join('')
  }

  if (typeof element === 'object' && element.$$typeof === Symbol.for('react.element')) {
    const { type, props } = element

    if (typeof type === 'string') {
      return renderHTMLElement(type, props, isStatic)
    }

    if (typeof type === 'function') {
      try {
        const result = type(props)
        return renderElementToString(result, isStatic)
      }
      catch (error) {
        if (error && typeof error.then === 'function') {
          throw error
        }
        throw error
      }
    }

    if (type === globalThis.Fragment || type === Symbol.for('react.fragment')) {
      return renderElementToString(props.children, isStatic)
    }
  }

  return ''
}

function renderHTMLElement(type, props, isStatic) {
  const { children, dangerouslySetInnerHTML, ...attributes } = props || {}

  if (dangerouslySetInnerHTML && dangerouslySetInnerHTML.__html) {
    if (['img', 'br', 'hr', 'input', 'meta', 'link'].includes(type)) {
      const attrs = renderAttributes(attributes, isStatic)
      return `<${type}${attrs} />`
    }

    const attrs = renderAttributes(attributes, isStatic)
    return `<${type}${attrs}>${dangerouslySetInnerHTML.__html}</${type}>`
  }

  if (['img', 'br', 'hr', 'input', 'meta', 'link'].includes(type)) {
    const attrs = renderAttributes(attributes, isStatic)
    return `<${type}${attrs} />`
  }

  const attrs = renderAttributes(attributes, isStatic)
  const childrenString = renderElementToString(children, isStatic)

  return `<${type}${attrs}>${childrenString}</${type}>`
}

function renderAttributes(attributes, _isStatic) {
  if (!attributes)
    return ''

  return Object.entries(attributes)
    .filter(([key, value]) => {
      if (key === 'key' || key === 'ref')
        return false
      if (key.startsWith('__'))
        return false
      return value !== null && value !== undefined && value !== false
    })
    .map(([key, value]) => {
      if (value === true)
        return ` ${key}`
      if (key === 'className')
        key = 'class'
      if (key === 'htmlFor')
        key = 'for'
      return ` ${key}="${escapeHtml(String(value))}"`
    })
    .join('')
}

function escapeHtml(text) {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

if (typeof globalThis.__resolved_promises === 'undefined') {
  globalThis.__resolved_promises = new Map()
}
globalThis.__current_suspense_depth = 0

globalThis.React = {
  createElement: globalThis.createElement,
  Fragment: globalThis.Fragment,
  Suspense: globalThis.Suspense,
  createContext: globalThis.createContext,
  useContext: globalThis.useContext,
  memo: globalThis.memo,
  forwardRef: globalThis.forwardRef,
  createRef: globalThis.createRef,
  use: globalThis.use,
  lazy: globalThis.lazy,
  StrictMode: globalThis.StrictMode,
  useState: globalThis.useState,
  useEffect: globalThis.useEffect,
  useRef: globalThis.useRef,
  useCallback: globalThis.useCallback,
  useMemo: globalThis.useMemo,
  useTransition: globalThis.useTransition,
  useDeferredValue: globalThis.useDeferredValue,
  useId: globalThis.useId,
  startTransition: globalThis.startTransition,
  flushSync: globalThis.flushSync,
  unstable_act: globalThis.unstable_act,
}

if (!globalThis.createElement) {
  throw new Error('createElement polyfill failed to initialize')
}

if (!globalThis.ReactDOMServer?.renderToString) {
  throw new Error('ReactDOMServer.renderToString polyfill failed to initialize')
}
