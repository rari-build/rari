/* eslint-disable no-undef */
if (typeof globalThis.React === 'undefined') {
  React = {
    createElement(type, props, ...children) {
      return { $$typeof: Symbol.for('react.element'), type, props: props || {}, children }
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

if (typeof globalThis.Component === 'undefined') {
  globalThis.Component = globalThis.React.Component
}

if (typeof globalThis.Suspense === 'undefined') {
  globalThis.Suspense = globalThis.React.Suspense
}

if (typeof globalThis.Fragment === 'undefined') {
  globalThis.Fragment = globalThis.React.Fragment
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
