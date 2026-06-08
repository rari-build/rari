const TSX_DEFAULT_REGEX = /\/([^/]+)\.tsx#/
const REACT_ELEMENT_TYPE = Symbol.for('react.transitional.element')
const REACT_CLIENT_REFERENCE = Symbol.for('react.client.reference')
const REACT_SUSPENSE_TYPE = Symbol.for('react.suspense')
const REACT_FRAGMENT_TYPE = Symbol.for('react.fragment')
const REACT_PROVIDER_TYPE = Symbol.for('react.provider')
const REACT_CONSUMER_TYPE = Symbol.for('react.consumer')

if (typeof globalThis !== 'undefined') {
  globalThis['~rsc'] = globalThis['~rsc'] || {}
  if (typeof globalThis['~rsc'].keyCounter === 'undefined')
    globalThis['~rsc'].keyCounter = 0
}

function pushPendingPromise(item) {
  const suspense = globalThis['~suspense']
  suspense.pendingPromises ??= []
  suspense.pendingPromises.push(item)
  suspense.pendingPromisesById ??= {}
  suspense.pendingPromisesById[item.id] = item

  if (item.boundaryId) {
    suspense.pendingPromisesByBoundary ??= {}
    suspense.pendingPromisesByBoundary[item.boundaryId] ??= []
    suspense.pendingPromisesByBoundary[item.boundaryId].push(item)
  }
}

if (typeof globalThis !== 'undefined') {
  const s = globalThis['~suspense'] ??= {}
  s.streaming ??= true
  s.promises ??= {}
  s.boundaryProps ??= {}
  s.discoveredBoundaries ??= []
  s.pendingPromises ??= []
  s.pendingPromisesById ??= {}
  s.pendingPromisesByBoundary ??= {}
  s.currentBoundaryId ??= null
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
    const isInSuspense = globalThis['~suspense'].currentBoundaryId

    if (isInSuspense) {
      const promiseId = `promise:${globalThis['~rsc'].keyCounter++}`
      globalThis['~suspense'].promises[promiseId] = element
      pushPendingPromise({
        id: promiseId,
        boundaryId: isInSuspense,
        componentPath: 'AsyncPromise',
      })
      return null
    }

    element = await element
  }

  if (
    typeof element === 'string'
    || typeof element === 'number'
    || typeof element === 'boolean'
  ) {
    if (typeof element === 'string' && element.length > 0 && element[0] === '$') {
      return `$${element}`
    }

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
    && element.$$typeof === REACT_ELEMENT_TYPE
    && element.type === REACT_FRAGMENT_TYPE
  ) {
    return await traverseToRsc(element.props.children, clientComponents, depth + 1)
  }

  if (
    element
    && typeof element === 'object'
    && element.$$typeof === REACT_ELEMENT_TYPE
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
        $$typeof: REACT_ELEMENT_TYPE,
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
        $$typeof: REACT_ELEMENT_TYPE,
        type: 'suspense',
        props: mergedProps,
        key: element.key ?? null,
      }

      return await traverseReactElement(fakeSuspenseElement, clientComponents, depth + 1)
    }

    if (element['~rari_lazy'] === true) {
      return {
        '~rari_lazy': true,
        '~rari_promise_id': element['~rari_promise_id'],
        '~rari_component_id': element['~rari_component_id'],
        '~rari_loading_id': element['~rari_loading_id'],
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
    const suspense = globalThis['~suspense']
    suspense.pendingPromisesByBoundary[boundaryId] = []

    const previousBoundaryId = suspense.currentBoundaryId
    suspense.currentBoundaryId = boundaryId

    try {
      const safeFallback = props?.fallback
        ? await traverseToRsc(props.fallback, clientComponents, depth + 1)
        : null

      suspense.discoveredBoundaries.push({
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
            suspense.promises[promiseId] = child
            pushPendingPromise({
              id: promiseId,
              boundaryId,
              componentPath: 'AsyncComponent',
            })
            continue
          }

          if (child.$$typeof === REACT_ELEMENT_TYPE && typeof child.type === 'function') {
            try {
              const isAsyncMarker = child.type._isAsyncComponent && child.type._originalType
              const isAsync = isAsyncMarker
                || child.type.constructor.name === 'AsyncFunction'
                || child.type.toString().trim().startsWith('async ')

              if (isAsync) {
                const promiseId = `promise:${globalThis['~rsc'].keyCounter++}`

                const actualType = isAsyncMarker ? child.type._originalType : child.type

                pushPendingPromise({
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

          if (child.props && child.props.children && !isSuspenseComponent(child.type))
            detectAsyncComponents(child.props.children, depth + 1)
        }
      }

      detectAsyncComponents(processedChildren)

      const hasPendingPromises = suspense.pendingPromises.some(
        p => p.boundaryId === boundaryId,
      )

      let traversedChildren
      if (hasPendingPromises) {
        const isStreaming = globalThis['~rari']?.streaming?.enabled === true

        if (isStreaming) {
          if (!globalThis['~rari'].lazy)
            globalThis['~rari'].lazy = { pending: new Map(), resolved: new Map(), counter: 0 }

          const boundaryPromises = suspense.pendingPromises.filter(
            p => p.boundaryId === boundaryId,
          )

          for (const pending of boundaryPromises) {
            if (!pending.componentType && suspense.promises[pending.id]) {
              globalThis['~rari'].lazy.pending.set(pending.id, suspense.promises[pending.id])
            }
            else {
              globalThis['~rari'].lazy.pending.set(pending.id, {
                component: pending.componentType,
                props: pending.componentProps,
                isDeferred: true,
              })
            }
          }

          const lazyMarkers = boundaryPromises.map(pending => ({
            '~rari_lazy': true,
            '~rari_promise_id': pending.id,
            '~rari_component_id': pending.componentPath || 'AsyncComponent',
            '~rari_loading_id': '',
          }))

          traversedChildren = lazyMarkers.length === 1
            ? lazyMarkers[0]
            : lazyMarkers.length > 0 ? lazyMarkers : null
        }
        else {
          const boundaryPromises = suspense.pendingPromises.filter(
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
                console.error(`[rari] Error rendering async component ${pending.componentPath}:`, error)
                resolvedComponents.push(null)
              }
            }
          }

          suspense.pendingPromises = suspense.pendingPromises.filter(
            p => p.boundaryId !== boundaryId,
          )

          traversedChildren = resolvedComponents.length === 1 ? resolvedComponents[0] : resolvedComponents
        }
      }
      else {
        traversedChildren = await traverseToRsc(props?.children, clientComponents, depth + 1)
      }

      return [
        '$',
        'react.suspense',
        null,
        {
          ...props,
          '~boundaryId': boundaryId,
          'fallback': safeFallback,
          'children': traversedChildren,
        },
      ]
    }
    finally {
      suspense.currentBoundaryId = previousBoundaryId
    }
  }

  if (typeof type === 'function' && type._isAsyncComponent && type._originalType) {
    const asyncType = type._originalType
    const isInSuspense = globalThis['~suspense'].currentBoundaryId

    if (isInSuspense) {
      const promiseId = `promise:${globalThis['~rsc'].keyCounter++}`

      pushPendingPromise({
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
      console.warn('[rari] Unresolved client component:', type)
      return null
    }
  }

  if (type && typeof type === 'object' && Object.keys(type).length === 0) {
    console.warn('[rari] Component failed to load (empty object)')
    return null
  }

  if (isServerComponent(type)) {
    const isAsync = typeof type === 'function' && type.constructor.name === 'AsyncFunction'
    const isInSuspense = globalThis['~suspense'].currentBoundaryId

    if (isAsync && isInSuspense) {
      const promiseId = `promise:${globalThis['~rsc'].keyCounter++}`
      pushPendingPromise({
        id: promiseId,
        boundaryId: isInSuspense,
        componentPath: type.name || 'anonymous',
        componentType: type,
        componentProps: props || {},
      })
      return null
    }

    const rendered = await renderServerComponent(element)
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
    let rendered = type(props)

    if (rendered && typeof rendered.then === 'function')
      rendered = await rendered

    if (rendered === element)
      return null

    return await traverseToRsc(rendered, clientComponents, depth + 1)
  }

  // eslint-disable-next-line no-undef
  if (type === React.Fragment)
    return await traverseToRsc(props.children, clientComponents, depth + 1)

  if (type === REACT_FRAGMENT_TYPE)
    return await traverseToRsc(props.children, clientComponents, depth + 1)

  if (type && type.$$typeof === REACT_PROVIDER_TYPE)
    return await traverseToRsc(props.children, clientComponents, depth + 1)

  if (type && type.$$typeof === REACT_CONSUMER_TYPE)
    return await traverseToRsc(props.children, clientComponents, depth + 1)

  console.warn('[rari] Unknown component type:', type)
  return null
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

function isClientComponent(componentType, clientComponents) {
  if (
    componentType
    && componentType.$$typeof === REACT_CLIENT_REFERENCE
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
  if (componentType && componentType['~isServerComponent'])
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
    const reactClientSymbol = REACT_CLIENT_REFERENCE
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
      const match = componentType.match(TSX_DEFAULT_REGEX)
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

async function renderToRsc(element, clientComponents = {}) {
  return await traverseToRsc(element, clientComponents)
}

function isSuspenseComponent(type) {
  if (
    typeof React !== 'undefined'
    // eslint-disable-next-line no-undef
    && React.Suspense && type === React.Suspense
  ) {
    return true
  }

  if (type && type.$$typeof === REACT_SUSPENSE_TYPE)
    return true

  if (type === REACT_SUSPENSE_TYPE)
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
