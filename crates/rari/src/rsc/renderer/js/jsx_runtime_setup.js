globalThis.__jsx_runtime = globalThis.__jsx_runtime || {
  jsx(type, props, key) {
    const element = {
      $typeof: Symbol.for('react.element'),
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
    return globalThis.__jsx_runtime.jsx(type, props, key)
  },
}
