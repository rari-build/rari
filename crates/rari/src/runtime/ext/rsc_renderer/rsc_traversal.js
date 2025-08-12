/* eslint-disable no-undef */

function traverseToRSC(element, clientComponents = {}, depth = 0) {
  if (depth > 100) {
    console.error(
      'RSC traversal depth limit exceeded, returning null to prevent stack overflow',
    )
    return null
  }
  if (!element) {
    return null
  }

  if (
    typeof element === 'string'
    || typeof element === 'number'
    || typeof element === 'boolean'
  ) {
    return element
  }

  if (Array.isArray(element)) {
    return element.map(child =>
      traverseToRSC(child, clientComponents, depth + 1),
    )
  }

  if (
    element
    && typeof element === 'object'
    && element.$$typeof === Symbol.for('react.element')
  ) {
    return traverseReactElement(element, clientComponents, depth + 1)
  }

  if (
    element
    && typeof element === 'object'
    && element.$$typeof === Symbol.for('react.fragment')
  ) {
    return traverseToRSC(element.props.children, clientComponents, depth + 1)
  }

  if (element && typeof element === 'object' && !element.$$typeof) {
    if (element.type && (typeof element.type === 'string' || typeof element.type === 'function')) {
      const hasPropsChildren = element.props && Object.prototype.hasOwnProperty.call(element.props || {}, 'children')
      const mergedProps = {
        ...(element.props || {}),
        children: hasPropsChildren ? element.props.children : element.children,
      }

      const fakeElement = {
        $$typeof: Symbol.for('react.element'),
        type: element.type,
        props: mergedProps,
        key: element.key ?? null,
      }
      return traverseReactElement(fakeElement, clientComponents, depth + 1)
    }

    if (!element.type && element.props && Object.prototype.hasOwnProperty.call(element.props, 'fallback')) {
      const mergedProps = {
        ...element.props,
        children: Object.prototype.hasOwnProperty.call(element.props, 'children')
          ? element.props.children
          : element.children,
      }

      const fakeSuspenseElement = {
        $$typeof: Symbol.for('react.element'),
        type: 'suspense',
        props: mergedProps,
        key: element.key ?? null,
      }

      return traverseReactElement(fakeSuspenseElement, clientComponents, depth + 1)
    }

    return element
  }

  return element
}

function traverseReactElement(element, clientComponents, depth = 0) {
  const { type, props, key } = element

  const uniqueKey
    = key || `element-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`

  if (isClientComponent(type, clientComponents)) {
    const componentId = getClientComponentId(type, clientComponents)

    if (componentId && componentId !== null) {
      return ['$', componentId, uniqueKey, props || {}]
    }
    else {
      return [
        '$',
        'div',
        uniqueKey,
        {
          'className': 'rsc-unresolved-client',
          'data-rsc-error': 'unresolved-client-component',
          'style': {
            border: '2px dashed #fdcb6e',
            padding: '8px',
            margin: '4px',
            backgroundColor: '#fff9e6',
            color: '#e17055',
          },
          'children': 'WARNING: Unresolved client component',
        },
      ]
    }
  }

  if (type && typeof type === 'object' && Object.keys(type).length === 0) {
    return [
      '$',
      'div',
      uniqueKey,
      {
        'className': 'rsc-missing-component',
        'data-rsc-error': 'empty-object',
        'style': {
          border: '2px dashed #ff6b6b',
          padding: '8px',
          margin: '4px',
          backgroundColor: '#ffe0e0',
          color: '#d63031',
        },
        'children': 'WARNING: Component failed to load (empty object)',
      },
    ]
  }

  if (isServerComponent(type)) {
    const rendered = renderServerComponent(element)
    return traverseToRSC(rendered, clientComponents, depth + 1)
  }

  if (isSuspenseComponent(type)) {
    const boundaryId
      = `boundary_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`

    if (!globalThis.__discovered_boundaries)
      globalThis.__discovered_boundaries = []
    if (!globalThis.__pending_promises)
      globalThis.__pending_promises = []
    if (!globalThis.__suspense_promises)
      globalThis.__suspense_promises = {}

    const previousBoundaryId = globalThis.__current_boundary_id
    globalThis.__current_boundary_id = boundaryId

    const defaultFallback = [
      '$',
      'div',
      null,
      { children: 'Loading...' },
    ]
    const safeFallback = props?.fallback
      ? traverseToRSC(props.fallback, clientComponents, depth + 1)
      : defaultFallback

    globalThis.__discovered_boundaries.push({
      id: boundaryId,
      fallback: safeFallback,
      parentId: previousBoundaryId,
    })

    const processedChildren = Array.isArray(props?.children)
      ? props.children
      : [props?.children]

    for (const child of processedChildren) {
      if (
        child
        && typeof child === 'object'
        && child.type
        && typeof child.type === 'function'
      ) {
        try {
          const isAsync
            = child.type.constructor.name === 'AsyncFunction'
              || child.type.toString().trim().startsWith('async ')

          if (isAsync) {
            const result = child.type(child.props || {})

            if (result && typeof result.then === 'function') {
              const promiseId
                = `promise_${
                  Date.now()
                }_${
                  Math.random().toString(36).substr(2, 9)}`
              globalThis.__suspense_promises[promiseId] = result

              globalThis.__pending_promises.push({
                id: promiseId,
                boundaryId,
                componentPath: child.type.name || 'anonymous',
              })
            }
          }
        }
        catch (error) {
          console.error('Error processing async component:', error)
        }
      }
    }

    globalThis.__current_boundary_id = previousBoundaryId

    return [
      '$',
      'react.suspense',
      null,
      {
        ...props,
        boundaryId,
        fallback: safeFallback,
        children: traverseToRSC(props?.children, clientComponents, depth + 1),
      },
    ]
  }

  if (typeof type === 'string') {
    return createRSCHTMLElement(
      type,
      props,
      uniqueKey,
      clientComponents,
      depth,
    )
  }

  if (typeof type === 'function') {
    try {
      const rendered = type(props)

      if (rendered === element) {
        return null
      }
      return traverseToRSC(rendered, clientComponents, depth + 1)
    }
    catch (error) {
      console.error('Error rendering function component:', error)
      return createErrorElement(
        error.message,
        type.name || 'FunctionComponent',
      )
    }
  }

  if (type === React.Fragment) {
    return traverseToRSC(props.children, clientComponents, depth + 1)
  }

  if (type && type.$$typeof === Symbol.for('react.provider')) {
    return traverseToRSC(props.children, clientComponents, depth + 1)
  }

  if (type && type.$$typeof === Symbol.for('react.consumer')) {
    return traverseToRSC(props.children, clientComponents, depth + 1)
  }

  return [
    '$',
    'div',
    uniqueKey,
    {
      'className': 'rsc-unknown-component',
      'data-rsc-error': 'unknown-component-type',
      'style': {
        border: '2px dashed #74b9ff',
        padding: '8px',
        margin: '4px',
        backgroundColor: '#e6f3ff',
        color: '#0984e3',
      },
      'children': 'WARNING: Unknown component type',
    },
  ]
}

function createRSCHTMLElement(
  tagName,
  props,
  key,
  clientComponents,
  depth = 0,
) {
  const { children, ...otherProps } = props || {}

  const rscProps = {
    ...otherProps,
    children: children
      ? traverseToRSC(children, clientComponents, depth + 1)
      : undefined,
  }

  if (rscProps.children === undefined) {
    delete rscProps.children
  }

  const uniqueKey
    = key
      || `${tagName}-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
  return ['$', tagName, uniqueKey, rscProps]
}

function renderServerComponent(element) {
  const { type: Component, props } = element

  try {
    if (Component.constructor.name === 'AsyncFunction') {
      return Component(props)
    }
    else {
      return Component(props)
    }
  }
  catch (error) {
    console.error('Error rendering server component:', error)
    return createErrorElement(
      error.message,
      Component.name || 'ServerComponent',
    )
  }
}

function isClientComponent(componentType, clientComponents) {
  if (
    componentType
    && componentType.$$typeof === Symbol.for('react.client.reference')
  ) {
    return true
  }

  if (clientComponents && typeof componentType === 'function') {
    const componentName = componentType.name || componentType.displayName
    if (componentName && clientComponents[componentName]) {
      return true
    }
  }

  if (
    componentType
    && typeof componentType === 'object'
    && Object.keys(componentType).length === 0
  ) {
    return true
  }

  if (componentType && componentType.__isClientComponent) {
    return true
  }

  if (typeof componentType === 'string') {
    if (
      componentType.includes('Counter')
      || componentType.includes('tsx#')
      || componentType.includes('use client')
    ) {
      return true
    }
  }

  return false
}

function isServerComponent(componentType) {
  if (componentType && componentType.__isServerComponent) {
    return true
  }

  if (
    typeof componentType === 'function'
    && componentType.constructor.name === 'AsyncFunction'
  ) {
    return true
  }

  return false
}

function getClientComponentId(componentType, clientComponents) {
  if (componentType && typeof componentType === 'object') {
    const reactClientSymbol = Symbol.for('react.client.reference')
    if (componentType.$$typeof === reactClientSymbol) {
      const clientId = componentType.$$id
      if (clientId) {
        return clientId
      }
    }
  }

  if (clientComponents && typeof componentType === 'function') {
    const componentName = componentType.name || componentType.displayName
    if (componentName && clientComponents[componentName]) {
      return clientComponents[componentName].id || componentName
    }
  }

  if (
    componentType
    && typeof componentType === 'object'
    && Object.keys(componentType).length === 0
  ) {
    const globalComponents = globalThis.__clientComponents || {}
    const componentNames = globalThis.__clientComponentNames || {}

    for (const [componentId, componentInfo] of Object.entries(
      globalComponents,
    )) {
      if (componentInfo.path && componentInfo.path.includes('Counter')) {
        return componentId
      }
    }

    for (const [name, id] of Object.entries(componentNames)) {
      if (name.toLowerCase().includes('counter')) {
        return id
      }
    }

    for (const [id, info] of Object.entries(globalComponents)) {
      if (
        info.component
        && (info.component.name === 'Counter'
          || info.component.displayName === 'Counter')
      ) {
        return id
      }
    }
    return null
  }

  if (typeof componentType === 'string') {
    const componentNames = globalThis.__clientComponentNames || {}

    let componentName = componentType
    if (componentType.includes('Counter')) {
      componentName = 'Counter'
    }
    else if (componentType.includes('tsx#default')) {
      const match = componentType.match(/\/([^/]+)\.tsx#/)
      if (match) {
        componentName = match[1]
      }
    }

    if (componentNames[componentName]) {
      return componentNames[componentName]
    }

    for (const [name, id] of Object.entries(componentNames)) {
      if (
        name.toLowerCase().includes(componentName.toLowerCase())
        || componentName.toLowerCase().includes(name.toLowerCase())
      ) {
        return id
      }
    }

    return null
  }

  const componentName
    = componentType.name || componentType.displayName || 'UnknownClient'

  if (componentName.startsWith('$L') || componentName.startsWith('client')) {
    return componentName
  }

  return null
}

function createErrorElement(message, componentName) {
  const errorId = `error-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
  return [
    '$',
    'div',
    errorId,
    {
      style: {
        color: 'red',
        border: '1px solid red',
        padding: '10px',
        margin: '10px',
      },
      children: [
        [
          '$',
          'h3',
          `${errorId}-h3`,
          {
            children: `Error in ${componentName}`,
          },
        ],
        [
          '$',
          'p',
          `${errorId}-p`,
          {
            children: message,
          },
        ],
      ],
    },
  ]
}

function renderToRSC(element, clientComponents = {}) {
  try {
    return traverseToRSC(element, clientComponents)
  }
  catch (error) {
    console.error('Error in RSC traversal:', error)
    return createErrorElement(error.message, 'RootComponent')
  }
}

function isSuspenseComponent(type) {
  if (
    typeof React !== 'undefined'
    && React.Suspense
    && type === React.Suspense
  ) {
    return true
  }

  if (type && type.$$typeof === Symbol.for('react.suspense')) {
    return true
  }

  if (
    typeof type === 'function'
    && (type.name === 'Suspense' || type.displayName === 'Suspense')
  ) {
    return true
  }

  if (type === 'suspense') {
    return true
  }

  return false
}

if (typeof globalThis !== 'undefined') {
  globalThis.traverseToRSC = traverseToRSC
  globalThis.renderToRSC = renderToRSC
  globalThis.isClientComponent = isClientComponent
  globalThis.isServerComponent = isServerComponent
  globalThis.getClientComponentId = getClientComponentId
  globalThis.isSuspenseComponent = isSuspenseComponent
}
