/* eslint-disable react/no-unnecessary-use-prefix  */
class ReactComponent {
  constructor(props) {
    this.props = props
    this.state = {}
  }

  setState(updater) {
    if (typeof updater === 'function')
      this.state = { ...this.state, ...updater(this.state, this.props) }
    else
      this.state = { ...this.state, ...updater }
  }

  render() {
    return null
  }
}

if (typeof globalThis.React === 'undefined') {
  globalThis.React = {
    createElement(type, props, ...children) {
      return { $$typeof: Symbol.for('react.transitional.element'), type, props: props || {}, children }
    },
    Fragment(props) { return props?.children || null },
    Suspense(props) { return props?.children || props?.fallback || null },
    Component: ReactComponent,
  }
}

if (!globalThis.React.Component)
  globalThis.React.Component = ReactComponent

if (typeof globalThis.Component === 'undefined')
  globalThis.Component = globalThis.React.Component

if (typeof globalThis.Suspense === 'undefined')
  globalThis.Suspense = globalThis.React.Suspense

if (typeof globalThis.Fragment === 'undefined')
  globalThis.Fragment = globalThis.React.Fragment

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

if (typeof globalThis.React !== 'undefined' && typeof globalThis.React.use !== 'function') {
  globalThis.React.use = function use(resource) {
    if (resource && resource.$$typeof === Symbol.for('react.context')) {
      console.warn('[rari] React.use() with Context is not fully supported on server')
      return null
    }

    if (resource && typeof resource === 'object' && typeof resource.then === 'function') {
      if (resource.status === 'fulfilled')
        return resource.value

      if (resource.status === 'rejected')
        throw resource.reason

      if (!resource.status) {
        resource.status = 'pending'

        resource.then(
          (value) => {
            if (resource.status === 'pending') {
              resource.status = 'fulfilled'
              resource.value = value
            }
          },
          (reason) => {
            if (resource.status === 'pending') {
              resource.status = 'rejected'
              resource.reason = reason
            }
          },
        )
      }

      throw resource
    }

    return resource
  }
}
