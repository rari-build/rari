/* eslint-disable no-undef */
if (typeof globalThis !== 'undefined') {
  globalThis['~rsc'] = globalThis['~rsc'] || {}
  if (typeof globalThis['~rsc'].keyCounter === 'undefined')
    globalThis['~rsc'].keyCounter = 0
}

if (typeof globalThis !== 'undefined' && !globalThis['~suspense']) {
  globalThis['~suspense'] = {
    streaming: true,
    promises: {},
    boundaryProps: {},
    discoveredBoundaries: [],
    pendingPromises: [],
    currentBoundaryId: null,
  }
}

async function traverseToRsc(element, clientComponents = {}, depth = 0) {
  if (depth > 100) {
    console.error(
      'RSC traversal depth limit exceeded, returning null to prevent stack overflow',
    )
    return null
  }
  if (!element)
    return null

  if (element && typeof element === 'object' && typeof element.then === 'function') {
    const isInSuspense = globalThis['~suspense']?.currentBoundaryId

    if (isInSuspense) {
      const promiseId = `promise:${globalThis['~rsc'].keyCounter++}`

      if (!globalThis['~suspense'].pendingPromises)
        globalThis['~suspense'].pendingPromises = []

      if (!globalThis['~suspense'].promises)
        globalThis['~suspense'].promises = {}

      globalThis['~suspense'].promises[promiseId] = element
      globalThis['~suspense'].pendingPromises.push({
        id: promiseId,
        boundaryId: globalThis['~suspense'].currentBoundaryId,
        componentPath: 'AsyncPromise',
      })

      return null
    }

    try {
      element = await element
    }
    catch (error) {
      console.error('Error awaiting Promise in RSC traversal:', error)
      return createErrorElement(
        error.message || String(error),
        'AsyncComponent',
      )
    }
  }

  if (
    typeof element === 'string'
    || typeof element === 'number'
    || typeof element === 'boolean'
  ) {
    return element
  }

  if (Array.isArray(element)) {
    const results = []
    for (const child of element)
      results.push(await traverseToRsc(child, clientComponents, depth + 1))

    return results
  }

  if (
    element
    && typeof element === 'object'
    && element.$$typeof === Symbol.for('react.transitional.element')
    && element.type === Symbol.for('react.fragment')
  ) {
    return await traverseToRsc(element.props.children, clientComponents, depth + 1)
  }

  if (
    element
    && typeof element === 'object'
    && element.$$typeof === Symbol.for('react.transitional.element')
  ) {
    return await traverseReactElement(element, clientComponents, depth + 1)
  }

  if (element && typeof element === 'object' && !element.$$typeof) {
    if (element['~preSerializedSuspense'] && element.rscArray)
      return element.rscArray

    if (element.type && (typeof element.type === 'string' || typeof element.type === 'function' || typeof element.type === 'object')) {
      const hasPropsChildren = element.props && Object.hasOwn(element.props || {}, 'children')
      const mergedProps = {
        ...(element.props || {}),
        children: hasPropsChildren ? element.props.children : element.children,
      }

      const fakeElement = {
        $$typeof: Symbol.for('react.transitional.element'),
        type: element.type,
        props: mergedProps,
        key: element.key ?? null,
      }
      return await traverseReactElement(fakeElement, clientComponents, depth + 1)
    }

    if (!element.type && element.props && Object.hasOwn(element.props, 'fallback')) {
      const mergedProps = {
        ...element.props,
        children: Object.hasOwn(element.props, 'children')
          ? element.props.children
          : element.children,
      }

      const fakeSuspenseElement = {
        $$typeof: Symbol.for('react.transitional.element'),
        type: 'suspense',
        props: mergedProps,
        key: element.key ?? null,
      }

      return await traverseReactElement(fakeSuspenseElement, clientComponents, depth + 1)
    }

    if (element.__rari_lazy === true) {
      return {
        __rari_lazy: true,
        __rari_promise_id: element.__rari_promise_id,
        __rari_component_id: element.__rari_component_id,
        __rari_loading_id: element.__rari_loading_id,
      }
    }

    return null
  }

  return element
}

async function traverseReactElement(element, clientComponents, depth = 0) {
  const { type, props, key } = element

  const uniqueKey = key || `element:${globalThis['~rsc'].keyCounter++}`

  if (isSuspenseComponent(type)) {
    const boundaryId = props?.['~boundaryId'] || `boundary:${globalThis['~rsc'].keyCounter++}`

    if (!globalThis['~suspense'])
      globalThis['~suspense'] = {}
    if (!globalThis['~suspense'].discoveredBoundaries)
      globalThis['~suspense'].discoveredBoundaries = []
    if (!globalThis['~suspense'].pendingPromises)
      globalThis['~suspense'].pendingPromises = []
    if (!globalThis['~suspense'].promises)
      globalThis['~suspense'].promises = {}

    const previousBoundaryId = globalThis['~suspense'].currentBoundaryId
    globalThis['~suspense'].currentBoundaryId = boundaryId

    const defaultFallback = null
    const safeFallback = props?.fallback
      ? await traverseToRsc(props.fallback, clientComponents, depth + 1)
      : defaultFallback

    globalThis['~suspense'].discoveredBoundaries.push({
      id: boundaryId,
      fallback: safeFallback,
      parentId: previousBoundaryId,
    })

    const processedChildren = Array.isArray(props?.children)
      ? props.children
      : [props?.children]

    function detectAsyncComponents(children, depth = 0) {
      if (depth > 10)
        return

      const childArray = Array.isArray(children) ? children : [children]

      for (const child of childArray) {
        if (!child || typeof child !== 'object')
          continue

        if (typeof child.then === 'function') {
          const promiseId = `promise:${globalThis['~rsc'].keyCounter++}`

          if (!globalThis['~suspense'].promises)
            globalThis['~suspense'].promises = {}

          globalThis['~suspense'].promises[promiseId] = child
          globalThis['~suspense'].pendingPromises.push({
            id: promiseId,
            boundaryId,
            componentPath: 'AsyncComponent',
          })

          continue
        }

        if (((child.$typeof === Symbol.for('react.transitional.element') || child.$$typeof === Symbol.for('react.transitional.element')) || child.$$typeof === Symbol.for('react.element')) && typeof child.type === 'function') {
          try {
            const isAsyncMarker = child.type._isAsyncComponent && child.type._originalType
            const isAsync = isAsyncMarker
              || child.type.constructor.name === 'AsyncFunction'
              || child.type.toString().trim().startsWith('async ')

            if (isAsync) {
              const promiseId = `promise:${globalThis['~rsc'].keyCounter++}`

              const actualType = isAsyncMarker ? child.type._originalType : child.type

              globalThis['~suspense'].pendingPromises.push({
                id: promiseId,
                boundaryId,
                componentPath: actualType.name || 'anonymous',
                componentType: actualType,
                componentProps: child.props || {},
              })
            }
          }
          catch (error) {
            console.error('Error detecting async component:', error)
          }
        }

        if (child.props && child.props.children)
          detectAsyncComponents(child.props.children, depth + 1)
      }
    }

    detectAsyncComponents(processedChildren)

    const hasPendingPromises = globalThis['~suspense'].pendingPromises.some(
      p => p.boundaryId === boundaryId,
    )

    const hasLazyMarker = processedChildren.some((child) => {
      const isLazy = child && typeof child === 'object' && child.__rari_lazy === true
      return isLazy
    })

    let traversedChildren
    if (hasPendingPromises) {
      const isStreaming = globalThis['~suspense']?.streaming === true || globalThis.__RARI_STREAMING_SUSPENSE__ === true

      if (isStreaming) {
        if (!globalThis.__RARI_PENDING_PROMISES__)
          globalThis.__RARI_PENDING_PROMISES__ = new Map()

        const boundaryPromises = globalThis['~suspense'].pendingPromises.filter(
          p => p.boundaryId === boundaryId,
        )

        for (const pending of boundaryPromises) {
          globalThis.__RARI_PENDING_PROMISES__.set(pending.id, {
            component: pending.componentType,
            props: pending.componentProps,
            isDeferred: true,
          })
        }

        const lazyMarkers = []
        for (const pending of boundaryPromises) {
          lazyMarkers.push({
            __rari_lazy: true,
            __rari_promise_id: pending.id,
            __rari_component_id: pending.componentPath || 'AsyncComponent',
            __rari_loading_id: '',
          })
        }

        traversedChildren = lazyMarkers.length === 1 ? lazyMarkers[0] : (lazyMarkers.length > 0 ? lazyMarkers : null)
      }
      else {
        const boundaryPromises = globalThis['~suspense'].pendingPromises.filter(
          p => p.boundaryId === boundaryId,
        )

        const resolvedComponents = []
        for (const pending of boundaryPromises) {
          if (pending.componentType && pending.componentProps !== undefined) {
            try {
              const result = await pending.componentType(pending.componentProps)
              const traversed = await traverseToRsc(result, clientComponents, depth + 1)
              resolvedComponents.push(traversed)
            }
            catch (error) {
              console.error('Error rendering async component:', error)
              resolvedComponents.push(createErrorElement(error.message, pending.componentPath))
            }
          }
        }

        globalThis['~suspense'].pendingPromises = globalThis['~suspense'].pendingPromises.filter(
          p => p.boundaryId !== boundaryId,
        )

        traversedChildren = resolvedComponents.length === 1 ? resolvedComponents[0] : resolvedComponents
      }
    }
    else if (hasLazyMarker) {
      traversedChildren = await traverseToRsc(props?.children, clientComponents, depth + 1)
    }
    else {
      traversedChildren = await traverseToRsc(props?.children, clientComponents, depth + 1)
    }

    globalThis['~suspense'].currentBoundaryId = previousBoundaryId

    const rscProps = {
      ...props,
      '~boundaryId': boundaryId,
      'fallback': safeFallback,
      'children': traversedChildren,
    }

    return [
      '$',
      'react.suspense',
      null,
      rscProps,
    ]
  }

  if (typeof type === 'function' && type._isAsyncComponent && type._originalType) {
    const asyncType = type._originalType
    const isInSuspense = globalThis['~suspense']?.currentBoundaryId

    if (isInSuspense) {
      const promiseId = `promise:${globalThis['~rsc'].keyCounter++}`

      if (!globalThis['~suspense'].pendingPromises)
        globalThis['~suspense'].pendingPromises = []

      globalThis['~suspense'].pendingPromises.push({
        id: promiseId,
        boundaryId: globalThis['~suspense'].currentBoundaryId,
        componentPath: asyncType.name || 'AsyncComponent',
        componentType: asyncType,
        componentProps: props || {},
      })

      return null
    }

    const result = await asyncType(props)
    return await traverseToRsc(result, clientComponents, depth + 1)
  }

  if (isClientComponent(type, clientComponents)) {
    const componentId = getClientComponentId(type, clientComponents)

    if (componentId && componentId !== null) {
      if (componentId.includes('#')) {
        const [filePath, exportName] = componentId.split('#')
        if (typeof globalThis['~rari']?.bridge !== 'undefined'
          && typeof globalThis['~rari'].bridge.registerClientReference === 'function') {
          try {
            globalThis['~rari'].bridge.registerClientReference(componentId, filePath, exportName)
          }
          catch (error) {
            console.error('Failed to register client reference:', error)
          }
        }
      }

      const processedProps = {}
      if (props) {
        for (const [key, value] of Object.entries(props)) {
          if (key === 'children')
            processedProps[key] = await traverseToRsc(value, clientComponents, depth + 1)
          else
            processedProps[key] = value
        }
      }
      return ['$', componentId, uniqueKey, processedProps]
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
    const isAsync = typeof type === 'function' && type.constructor.name === 'AsyncFunction'
    const isInSuspense = globalThis['~suspense']?.currentBoundaryId

    if (isAsync && isInSuspense) {
      const promiseId = `promise:${globalThis['~rsc'].keyCounter++}`

      if (!globalThis['~suspense'].pendingPromises)
        globalThis['~suspense'].pendingPromises = []

      globalThis['~suspense'].pendingPromises.push({
        id: promiseId,
        boundaryId: globalThis['~suspense'].currentBoundaryId,
        componentPath: type.name || 'anonymous',
        componentType: type,
        componentProps: props || {},
      })

      return null
    }

    const rendered = renderServerComponent(element)
    return await traverseToRsc(rendered, clientComponents, depth + 1)
  }

  if (typeof type === 'string') {
    return await createRSCHTMLElement(
      type,
      props,
      uniqueKey,
      clientComponents,
      depth,
    )
  }

  if (typeof type === 'function') {
    try {
      let rendered = type(props)

      if (rendered && typeof rendered.then === 'function')
        rendered = await rendered

      if (rendered === element)
        return null
      return await traverseToRsc(rendered, clientComponents, depth + 1)
    }
    catch (error) {
      console.error('Error rendering function component:', error)
      return createErrorElement(
        error.message,
        type.name || 'FunctionComponent',
      )
    }
  }

  if (type === React.Fragment)
    return await traverseToRsc(props.children, clientComponents, depth + 1)

  if (type === Symbol.for('react.fragment'))
    return await traverseToRsc(props.children, clientComponents, depth + 1)

  if (type && type.$$typeof === Symbol.for('react.provider'))
    return await traverseToRsc(props.children, clientComponents, depth + 1)

  if (type && type.$$typeof === Symbol.for('react.consumer'))
    return await traverseToRsc(props.children, clientComponents, depth + 1)

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

async function createRSCHTMLElement(
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
      ? await traverseToRsc(children, clientComponents, depth + 1)
      : undefined,
  }

  if (rscProps.children === undefined || rscProps.children === null)
    delete rscProps.children

  const uniqueKey = key || `${tagName}:${globalThis['~rsc'].keyCounter++}`
  return ['$', tagName, uniqueKey, rscProps]
}

async function renderServerComponent(element) {
  const { type: Component, props } = element

  try {
    let result
    if (Component.constructor.name === 'AsyncFunction') {
      result = await Component(props)
    }
    else {
      result = Component(props)
      if (result && typeof result.then === 'function')
        result = await result
    }
    return result
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
    if (componentName && clientComponents[componentName])
      return true
  }

  if (
    componentType
    && typeof componentType === 'object'
    && Object.keys(componentType).length === 0
  ) {
    return true
  }

  if (componentType && componentType['~isClientComponent'])
    return true

  if (typeof componentType === 'string') {
    if (
      componentType.includes('tsx#')
      || componentType.includes('use client')
    ) {
      return true
    }
  }

  return false
}

function isServerComponent(componentType) {
  if (componentType && componentType.__isServerComponent)
    return true

  if (
    typeof componentType === 'function'
    && componentType.constructor.name === 'AsyncFunction'
  ) {
    return true
  }

  return false
}

function getClientComponentId(componentType, clientComponents) {
  if (componentType && (typeof componentType === 'object' || typeof componentType === 'function')) {
    const reactClientSymbol = Symbol.for('react.client.reference')
    if (componentType.$$typeof === reactClientSymbol) {
      const clientId = componentType.$$id
      if (clientId)
        return clientId
    }
  }

  if (clientComponents && typeof componentType === 'function') {
    const componentName = componentType.name || componentType.displayName
    if (componentName && clientComponents[componentName])
      return clientComponents[componentName].id || componentName
  }

  if (
    componentType
    && typeof componentType === 'object'
    && Object.keys(componentType).length === 0
  ) {
    return null
  }

  if (typeof componentType === 'string') {
    const componentNames = globalThis['~clientComponentNames'] || {}

    let componentName = componentType
    if (componentType.includes('tsx#default')) {
      const match = componentType.match(/\/([^/]+)\.tsx#/)
      if (match)
        componentName = match[1]
    }

    if (componentNames[componentName])
      return componentNames[componentName]

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

  if (componentName.startsWith('$L') || componentName.startsWith('client'))
    return componentName

  return null
}

function createErrorElement(message, componentName) {
  const errorId = `error:${globalThis['~rsc'].keyCounter++}`
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

async function renderToRsc(element, clientComponents = {}) {
  try {
    globalThis['~rsc'].keyCounter = 0
    return await traverseToRsc(element, clientComponents)
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

  if (type && type.$$typeof === Symbol.for('react.suspense'))
    return true

  if (type === Symbol.for('react.suspense'))
    return true

  if (
    typeof type === 'function'
    && (type.name === 'Suspense' || type.displayName === 'Suspense')
  ) {
    return true
  }

  if (type === 'suspense')
    return true

  return false
}

if (typeof globalThis !== 'undefined') {
  globalThis.traverseToRsc = traverseToRsc
  globalThis.renderToRsc = renderToRsc
  globalThis.isClientComponent = isClientComponent
  globalThis.isServerComponent = isServerComponent
  globalThis.getClientComponentId = getClientComponentId
  globalThis.isSuspenseComponent = isSuspenseComponent
}
