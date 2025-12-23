if (!globalThis['~react'])
  globalThis['~react'] = {}
globalThis['~react'].jsxRuntime = globalThis['~react'].jsxRuntime || {
  jsx(type, props, key) {
    const element = {
      $$typeof: Symbol.for('react.transitional.element'),
      type,
      props: props || {},
      key: key || null,
      ref: null,
    }
    if (props && props.children !== undefined) {
      element.props = { ...element.props, children: props.children }
    }
    return element
  },
  jsxs(type, props, key) {
    return globalThis['~react'].jsxRuntime.jsx(type, props, key)
  },
}
