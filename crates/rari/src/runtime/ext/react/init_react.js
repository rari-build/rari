/* eslint-disable node/prefer-global/process */

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
  const suspenseId = `suspense-${Math.random().toString(36).substr(2, 9)}`

  if (globalThis.__suspense_manager) {
    globalThis.__suspense_manager.registerBoundary(suspenseId, fallback)
  }

  return globalThis.createElement(
    'suspense',
    {
      fallback,
      'data-suspense-id': suspenseId,
      'data-suspense-boundary': true,
    },
    children,
  )
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

globalThis.useState = function (initialValue) {
  const value
    = typeof initialValue === 'function' ? initialValue() : initialValue
  const setState = function () {
    throw new Error('setState called in server component')
  }
  return [value, setState]
}

globalThis.useEffect = function () {
  // No-op in SSR
}

globalThis.useLayoutEffect = function () {
  // No-op in SSR
}

globalThis.useCallback = function (callback) {
  return callback
}

globalThis.useMemo = function (factory) {
  return factory()
}

globalThis.useRef = function (initialValue) {
  return { current: initialValue }
}

globalThis.useImperativeHandle = function () {
  // No-op in SSR
}

globalThis.useDebugValue = function () {
  // No-op in SSR
}

globalThis.useContext = function (context) {
  if (!context || context.$$typeof !== Symbol.for('react.context')) {
    throw new Error('useContext must be called with a valid context')
  }
  return context._currentValue
}

globalThis.use = function use(resource) {
  if (resource && typeof resource.then === 'function') {
    const cached = globalThis.__promise_cache?.get(resource)
    if (cached?.resolved) {
      return cached.value
    }

    resource.$$typeof = Symbol.for('react.suspense.pending')
    resource.promise = resource

    throw resource
  }

  if (resource && resource.$$typeof === Symbol.for('react.context')) {
    return globalThis.use(resource)
  }

  throw new Error('use() called with unsupported resource type')
}

globalThis.forwardRef = function (render) {
  const forwardRefComponent = function (props) {
    return render(props, { current: null })
  }
  forwardRefComponent.$$typeof = Symbol.for('react.forward_ref')
  forwardRefComponent.render = render
  return forwardRefComponent
}

globalThis.memo = function (Component, compare) {
  const MemoComponent = function (props) {
    return Component(props)
  }
  MemoComponent.$$typeof = Symbol.for('react.memo')
  MemoComponent.type = Component
  MemoComponent.compare = compare
  return MemoComponent
}

globalThis.lazy = function (factory) {
  const LazyComponent = function () {
    throw new Error('React.lazy not supported in SSR')
  }
  LazyComponent.$$typeof = Symbol.for('react.lazy')
  LazyComponent._payload = factory
  LazyComponent._init = function (payload) {
    return payload()
  }
  return LazyComponent
}

globalThis.ErrorBoundary = function ErrorBoundary({ children }) {
  return children
}

globalThis.SuspenseManager = class SuspenseManager {
  constructor() {
    this.boundaries = new Map()
    this.promises = new Map()
    this.boundaryStack = []
    this.promiseCounter = 0
  }

  registerBoundary(id, fallback) {
    if (!this.boundaries.has(id)) {
      this.boundaries.set(id, {
        id,
        fallback,
        pending: new Set(),
        resolved: false,
        error: null,
        createdAt: Date.now(),
      })
    }
    return id
  }

  registerPromise(componentId, boundaryId, cacheKey) {
    const promiseId = `promise-${++this.promiseCounter}`
    const promiseInfo = {
      id: promiseId,
      componentId,
      boundaryId,
      cacheKey,
      status: 'pending',
      createdAt: Date.now(),
    }

    this.promises.set(promiseId, promiseInfo)

    if (boundaryId && this.boundaries.has(boundaryId)) {
      this.boundaries.get(boundaryId).pending.add(promiseId)
    }

    return promiseId
  }

  resolvePromise(promiseId, value) {
    const promiseInfo = this.promises.get(promiseId)
    if (!promiseInfo)
      return

    promiseInfo.status = 'resolved'
    promiseInfo.resolvedAt = Date.now()
    promiseInfo.value = value

    if (promiseInfo.boundaryId && this.boundaries.has(promiseInfo.boundaryId)) {
      const boundary = this.boundaries.get(promiseInfo.boundaryId)
      boundary.pending.delete(promiseId)

      if (boundary.pending.size === 0) {
        boundary.resolved = true
        boundary.resolvedAt = Date.now()
      }
    }
  }

  rejectPromise(promiseId, error) {
    const promiseInfo = this.promises.get(promiseId)
    if (!promiseInfo)
      return

    promiseInfo.status = 'rejected'
    promiseInfo.resolvedAt = Date.now()
    promiseInfo.error = error

    if (promiseInfo.boundaryId && this.boundaries.has(promiseInfo.boundaryId)) {
      const boundary = this.boundaries.get(promiseInfo.boundaryId)
      boundary.error = error
    }
  }

  getBoundaryState(boundaryId) {
    return this.boundaries.get(boundaryId)
  }

  getAllBoundaries() {
    return Array.from(this.boundaries.values())
  }

  reset() {
    this.boundaries.clear()
    this.promises.clear()
    this.boundaryStack = []
    this.promiseCounter = 0
  }
}

if (!globalThis.__suspense_manager) {
  globalThis.__suspense_manager = new globalThis.SuspenseManager()
}

if (!globalThis.__promise_cache) {
  globalThis.__promise_cache = new Map()
}

globalThis.React = {
  createElement: globalThis.createElement,
  Fragment: globalThis.Fragment,
  Suspense: globalThis.Suspense,
  createContext: globalThis.createContext,
  useState: globalThis.useState,
  useEffect: globalThis.useEffect,
  useLayoutEffect: globalThis.useLayoutEffect,
  useCallback: globalThis.useCallback,
  useMemo: globalThis.useMemo,
  useRef: globalThis.useRef,
  useImperativeHandle: globalThis.useImperativeHandle,
  useDebugValue: globalThis.useDebugValue,
  useContext: globalThis.useContext,
  use: globalThis.use,
  forwardRef: globalThis.forwardRef,
  memo: globalThis.memo,
  lazy: globalThis.lazy,
  ErrorBoundary: globalThis.ErrorBoundary,
}

globalThis.registerClientComponent = function (
  id,
  moduleRef,
  exportName = 'default',
) {
  if (!globalThis.__client_components) {
    globalThis.__client_components = new Map()
  }
  globalThis.__client_components.set(id, { moduleRef, exportName })
}

globalThis.getClientComponent = function (id) {
  return globalThis.__client_components?.get(id)
}

if (
  typeof globalThis.process !== 'undefined'
  && globalThis.process.env?.NODE_ENV === 'development'
) {
  globalThis.__dev_mode = true
}
