/* eslint-disable no-undef, style/object-curly-spacing */
(function () {
  const props = {page_props_json}
  const component = globalThis['{component_id}']
  if (!component || typeof component !== 'function') {
    throw new Error('Component not found: {component_id}')
  }

  const element = component(props)

  function serializeElement(el) {
    if (!el || typeof el !== 'object') {
      return el
    }

    if (Array.isArray(el)) {
      return el.map(serializeElement)
    }

    if (el.type !== undefined && el.props !== undefined) {
      const result = {
        type: el.type,
        props: {},
        key: el.key || null,
      }

      for (const [key, value] of Object.entries(el.props)) {
        if (key === 'children') {
          if (Array.isArray(value)) {
            result.props.children = value.map(serializeElement)
          }
          else {
            result.props.children = serializeElement(value)
          }
        }
        else {
          result.props[key] = value
        }
      }

      return result
    }

    return el
  }

  return serializeElement(element)
})()
