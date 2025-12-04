/* eslint-disable style/max-statements-per-line, no-use-before-define */
// oxlint-disable block-scoped-var, no-var, vars-on-top
if (typeof _jsx === 'undefined') {
  var _jsx = globalThis.__jsx_runtime?.jsx || (() => null)
}
if (typeof _jsxs === 'undefined') {
  var _jsxs = globalThis.__jsx_runtime?.jsxs || (() => null)
}

if (typeof globalThis.React === 'undefined') {
  globalThis.React = {
    createElement(type, props, ...children) {
      return { $typeof: Symbol.for('react.element'), type, props: props || {}, children }
    },
    Fragment(props) { return props?.children || null },
    Suspense(props) { return props?.children || props?.fallback || null },
    Component: class Component {
      constructor(props) {
        this.props = props
        this.state = {}
      }

      setState(updater) {
        if (typeof updater === 'function') {
          this.state = { ...this.state, ...updater(this.state, this.props) }
        }
        else {
          this.state = { ...this.state, ...updater }
        }
      }

      render() {
        return null
      }
    },
  }
}

if (!globalThis.React.Suspense) {
  globalThis.React.Suspense = function (props) { return props?.children || props?.fallback || null }
}

if (!globalThis.React.Component) {
  globalThis.React.Component = class Component {
    constructor(props) {
      this.props = props
      this.state = {}
    }

    setState(updater) {
      if (typeof updater === 'function') {
        this.state = { ...this.state, ...updater(this.state, this.props) }
      }
      else {
        this.state = { ...this.state, ...updater }
      }
    }

    render() {
      return null
    }
  }
}

if (typeof globalThis.Suspense === 'undefined') {
  globalThis.Suspense = globalThis.React.Suspense
}

if (typeof globalThis.Fragment === 'undefined') {
  globalThis.Fragment = globalThis.React.Fragment
}

if (typeof globalThis.Component === 'undefined') {
  globalThis.Component = globalThis.React.Component
}

if (typeof globalThis.jsx === 'undefined') {
  globalThis.jsx = function (type, props, key) {
    return globalThis.React.createElement(type, { ...props, key })
  }
}

if (typeof globalThis.jsxs === 'undefined') {
  globalThis.jsxs = function (type, props, key) {
    return globalThis.React.createElement(type, { ...props, key })
  }
}

if (typeof globalThis.LoadingSpinner === 'undefined') {
  globalThis.LoadingSpinner = function () {
    return globalThis.React.createElement('div', {
      style: {
        width: '40px',
        height: '40px',
        border: '4px solid #f3f4f6',
        borderTop: '4px solid #3b82f6',
        borderRadius: '50%',
        animation: 'spin 1s linear infinite',
      },
    })
  }
}

if (typeof globalThis.DefaultLoading === 'undefined') {
  globalThis.DefaultLoading = function () {
    return globalThis.React.createElement('div', {
      style: {
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        padding: '2rem',
        minHeight: '200px',
      },
    }, globalThis.React.createElement(globalThis.LoadingSpinner))
  }
}

if (!globalThis.readFileSync && globalThis.__nodeModules && globalThis.__nodeModules.get) {
  const nodeFs = globalThis.__nodeModules.get('node:fs')
  if (nodeFs && nodeFs.readFileSync) {
    globalThis.readFileSync = nodeFs.readFileSync
    globalThis.existsSync = nodeFs.existsSync
  }
  const nodePath = globalThis.__nodeModules.get('node:path')
  if (nodePath && nodePath.join) {
    globalThis.join = nodePath.join
  }
  const nodeProcess = globalThis.__nodeModules.get('node:process')
  if (nodeProcess && nodeProcess.cwd) {
    globalThis.cwd = nodeProcess.cwd
  }
}
