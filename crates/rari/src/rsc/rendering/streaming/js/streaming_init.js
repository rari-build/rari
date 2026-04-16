if (!globalThis['~rsc'])
  globalThis['~rsc'] = {}
if (typeof globalThis['~rsc'].keyCounter === 'undefined')
  globalThis['~rsc'].keyCounter = 0

if (typeof React === 'undefined')
  throw new TypeError('React is not available in streaming context. This suggests the runtime was not properly initialized with React extensions.')

if (typeof globalThis['~rsc'].renderGeneration === 'undefined')
  globalThis['~rsc'].renderGeneration = 0
globalThis['~rsc'].renderGeneration++

if (!globalThis['~suspense'])
  globalThis['~suspense'] = {}

Object.assign(globalThis['~suspense'], {
  streaming: true,
  promises: {},
  boundaryProps: {},
  discoveredBoundaries: [],
  pendingPromises: [],
  currentBoundaryId: null,
  renderGeneration: globalThis['~rsc'].renderGeneration,
})

globalThis['~suspense'].safeSerializeElement = function (element) {
  if (element == null)
    return null

  try {
    if (Array.isArray(element)) {
      if (element[0] === '$')
        return element

      return element.map(child => globalThis['~suspense'].safeSerializeElement(child))
    }

    if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean')
      return element

    if (element && typeof element === 'object') {
      const safeProps = {}

      if (element.props) {
        if (element.props.children !== undefined) {
          const children = element.props.children
          if (children === null || children === undefined) {
            safeProps.children = null
          }
          else if (Array.isArray(children)) {
            safeProps.children = children.map(child => globalThis['~suspense'].safeSerializeElement(child))
          }
          else if (typeof children === 'object') {
            safeProps.children = globalThis['~suspense'].safeSerializeElement(children)
          }
          else {
            safeProps.children = children
          }
        }
        else {
          safeProps.children = null
        }

        for (const key in element.props) {
          if (key === 'children' || key === 'key' || key === 'ref')
            continue

          const value = element.props[key]
          if (value === null || value === undefined)
            continue

          if (
            key === 'className'
            || key === 'style'
            || key === 'href'
            || key === 'src'
            || key === 'alt'
            || key === 'title'
            || key === 'id'
            || key === 'type'
            || key === 'placeholder'
            || key === 'value'
            || key === 'disabled'
            || key === 'checked'
            || key === 'selected'
            || key === 'readonly'
            || key === 'required'
            || key === 'htmlFor'
            || key === 'role'
            || key === 'name'
            || key === 'tabIndex'
            || key.startsWith('data-')
            || key.startsWith('aria-')
          ) {
            safeProps[key] = value
          }
        }
      }
      else {
        safeProps.children = null
      }

      return {
        type: typeof element.type === 'function'
          ? (element.type.name || 'div')
          : (element.type || 'div'),
        props: safeProps,
        key: null,
      }
    }

    return { type: 'div', props: { children: null }, key: null }
  }
  catch {
    return { type: 'div', props: { children: null }, key: null }
  }
}

// oxlint-disable no-unused-expressions
globalThis['~rsc'].renderGeneration
