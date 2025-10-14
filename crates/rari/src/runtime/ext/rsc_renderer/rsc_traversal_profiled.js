/* eslint-disable no-undef */

const perfStats = {
  totalCalls: 0,
  totalTime: 0,
  keyGenTime: 0,
  typeDetectionTime: 0,
  recursionTime: 0,
  promiseAwaitTime: 0,
  componentRenderTime: 0,
  arrayProcessingTime: 0,
  htmlElementTime: 0,
  suspenseTime: 0,
  clientComponentTime: 0,
  serverComponentTime: 0,
  functionComponentTime: 0,
  primitiveTime: 0,
  objectAllocationTime: 0,
  symbolLookupTime: 0,
}

function resetPerfStats() {
  Object.keys(perfStats).forEach(key => perfStats[key] = 0)
}

function getPerfStats() {
  return { ...perfStats }
}

function timeOperation(name, fn) {
  const start = performance.now()
  const result = fn()
  const elapsed = performance.now() - start
  perfStats[name] = (perfStats[name] || 0) + elapsed
  return result
}

async function timeOperationAsync(name, fn) {
  const start = performance.now()
  const result = await fn()
  const elapsed = performance.now() - start
  perfStats[name] = (perfStats[name] || 0) + elapsed
  return result
}

function generateUniqueKey(prefix = 'element') {
  return timeOperation('keyGenTime', () => {
    return `${prefix}-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
  })
}

function getReactSymbol(name) {
  return timeOperation('symbolLookupTime', () => {
    return Symbol.for(name)
  })
}

async function traverseToRSCProfiled(element, clientComponents = {}, depth = 0) {
  perfStats.totalCalls++
  const startTime = performance.now()

  try {
    if (depth > 100) {
      console.error('RSC traversal depth limit exceeded')
      return null
    }

    if (!element) {
      perfStats.primitiveTime += performance.now() - startTime
      return null
    }

    if (element && typeof element === 'object' && typeof element.then === 'function') {
      const result = await timeOperationAsync('promiseAwaitTime', async () => {
        try {
          return await element
        }
        catch (error) {
          console.error('Error awaiting Promise:', error)
          return createErrorElement(error.message || String(error), 'AsyncComponent')
        }
      })
      element = result
    }

    if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean') {
      perfStats.primitiveTime += performance.now() - startTime
      return element
    }

    if (Array.isArray(element)) {
      const result = await timeOperationAsync('arrayProcessingTime', async () => {
        const results = []
        for (const child of element) {
          results.push(await traverseToRSCProfiled(child, clientComponents, depth + 1))
        }
        return results
      })
      perfStats.totalTime += performance.now() - startTime
      return result
    }

    const isReactElement = timeOperation('typeDetectionTime', () => {
      return element && typeof element === 'object' && element.$typeof === getReactSymbol('react.element')
    })

    if (isReactElement) {
      const result = await timeOperationAsync('recursionTime', async () => {
        return await traverseReactElementProfiled(element, clientComponents, depth + 1)
      })
      perfStats.totalTime += performance.now() - startTime
      return result
    }

    const isFragment = timeOperation('typeDetectionTime', () => {
      return element && typeof element === 'object' && element.$typeof === getReactSymbol('react.fragment')
    })

    if (isFragment) {
      const result = await timeOperationAsync('recursionTime', async () => {
        return await traverseToRSCProfiled(element.props.children, clientComponents, depth + 1)
      })
      perfStats.totalTime += performance.now() - startTime
      return result
    }

    if (element && typeof element === 'object' && !element.$typeof) {
      if (element.type && (typeof element.type === 'string' || typeof element.type === 'function')) {
        const fakeElement = timeOperation('objectAllocationTime', () => {
          const hasPropsChildren = element.props && Object.prototype.hasOwnProperty.call(element.props || {}, 'children')
          return {
            $typeof: getReactSymbol('react.element'),
            type: element.type,
            props: {
              ...(element.props || {}),
              children: hasPropsChildren ? element.props.children : element.children,
            },
            key: element.key ?? null,
          }
        })

        const result = await timeOperationAsync('recursionTime', async () => {
          return await traverseReactElementProfiled(fakeElement, clientComponents, depth + 1)
        })
        perfStats.totalTime += performance.now() - startTime
        return result
      }
    }

    perfStats.totalTime += performance.now() - startTime
    return element
  }
  catch (error) {
    console.error('Error in traverseToRSCProfiled:', error)
    perfStats.totalTime += performance.now() - startTime
    return null
  }
}

async function traverseReactElementProfiled(element, clientComponents, depth = 0) {
  const startTime = performance.now()
  const { type, props, key } = element

  const uniqueKey = key || generateUniqueKey('element')

  const isClient = timeOperation('typeDetectionTime', () => {
    return isClientComponent(type, clientComponents)
  })

  if (isClient) {
    const result = timeOperation('clientComponentTime', () => {
      const componentId = getClientComponentId(type, clientComponents)
      if (componentId && componentId !== null) {
        return ['$', componentId, uniqueKey, props || {}]
      }
      else {
        return ['$', 'div', uniqueKey, {
          'className': 'rsc-unresolved-client',
          'data-rsc-error': 'unresolved-client-component',
          'children': 'WARNING: Unresolved client component',
        }]
      }
    })
    perfStats.totalTime += performance.now() - startTime
    return result
  }

  const isServer = timeOperation('typeDetectionTime', () => {
    return isServerComponent(type)
  })

  if (isServer) {
    const rendered = await timeOperationAsync('serverComponentTime', async () => {
      return renderServerComponent(element)
    })
    const result = await timeOperationAsync('recursionTime', async () => {
      return await traverseToRSCProfiled(rendered, clientComponents, depth + 1)
    })
    perfStats.totalTime += performance.now() - startTime
    return result
  }

  const isSuspense = timeOperation('typeDetectionTime', () => {
    return isSuspenseComponent(type)
  })

  if (isSuspense) {
    const result = await timeOperationAsync('suspenseTime', async () => {
      const boundaryId = generateUniqueKey('boundary')

      if (!globalThis.__discovered_boundaries)
        globalThis.__discovered_boundaries = []
      if (!globalThis.__pending_promises)
        globalThis.__pending_promises = []
      if (!globalThis.__suspense_promises)
        globalThis.__suspense_promises = {}

      const previousBoundaryId = globalThis.__current_boundary_id
      globalThis.__current_boundary_id = boundaryId

      const safeFallback = props?.fallback
        ? await traverseToRSCProfiled(props.fallback, clientComponents, depth + 1)
        : null

      globalThis.__discovered_boundaries.push({
        id: boundaryId,
        fallback: safeFallback,
        parentId: previousBoundaryId,
      })

      globalThis.__current_boundary_id = previousBoundaryId

      return ['$', 'react.suspense', null, {
        ...props,
        boundaryId,
        fallback: safeFallback,
        children: await traverseToRSCProfiled(props?.children, clientComponents, depth + 1),
      }]
    })
    perfStats.totalTime += performance.now() - startTime
    return result
  }

  if (typeof type === 'string') {
    const result = await timeOperationAsync('htmlElementTime', async () => {
      return await createRSCHTMLElementProfiled(type, props, uniqueKey, clientComponents, depth)
    })
    perfStats.totalTime += performance.now() - startTime
    return result
  }

  if (typeof type === 'function') {
    const result = await timeOperationAsync('functionComponentTime', async () => {
      try {
        let rendered = type(props)
        if (rendered && typeof rendered.then === 'function') {
          rendered = await rendered
        }
        if (rendered === element) {
          return null
        }
        return await traverseToRSCProfiled(rendered, clientComponents, depth + 1)
      }
      catch (error) {
        console.error('Error rendering function component:', error)
        return createErrorElement(error.message, type.name || 'FunctionComponent')
      }
    })
    perfStats.totalTime += performance.now() - startTime
    return result
  }

  if (type === React.Fragment) {
    const result = await timeOperationAsync('recursionTime', async () => {
      return await traverseToRSCProfiled(props.children, clientComponents, depth + 1)
    })
    perfStats.totalTime += performance.now() - startTime
    return result
  }

  perfStats.totalTime += performance.now() - startTime
  return ['$', 'div', uniqueKey, {
    'className': 'rsc-unknown-component',
    'data-rsc-error': 'unknown-component-type',
    'children': 'WARNING: Unknown component type',
  }]
}

async function createRSCHTMLElementProfiled(tagName, props, key, clientComponents, depth = 0) {
  const startTime = performance.now()

  const { children, ...otherProps } = props || {}

  const rscProps = timeOperation('objectAllocationTime', () => {
    return {
      ...otherProps,
      children: undefined,
    }
  })

  if (children) {
    rscProps.children = await timeOperationAsync('recursionTime', async () => {
      return await traverseToRSCProfiled(children, clientComponents, depth + 1)
    })
  }

  if (rscProps.children === undefined) {
    delete rscProps.children
  }

  const uniqueKey = key || generateUniqueKey(tagName)

  const result = timeOperation('objectAllocationTime', () => {
    return ['$', tagName, uniqueKey, rscProps]
  })

  perfStats.htmlElementTime += performance.now() - startTime
  return result
}

function renderServerComponent(element) {
  const { type: Component, props } = element
  try {
    let result
    if (Component.constructor.name === 'AsyncFunction') {
      result = Component(props)
    }
    else {
      result = Component(props)
      if (result && typeof result.then === 'function') {
        // Will be awaited by caller
      }
    }
    return result
  }
  catch (error) {
    console.error('Error rendering server component:', error)
    return createErrorElement(error.message, Component.name || 'ServerComponent')
  }
}

function createErrorElement(message, componentName) {
  const errorId = generateUniqueKey('error')
  return ['$', 'div', errorId, {
    style: { color: 'red', border: '1px solid red', padding: '10px', margin: '10px' },
    children: [
      ['$', 'h3', `${errorId}-h3`, { children: `Error in ${componentName}` }],
      ['$', 'p', `${errorId}-p`, { children: message }],
    ],
  }]
}

function isClientComponent(componentType, clientComponents) {
  if (componentType && componentType.$typeof === Symbol.for('react.client.reference')) {
    return true
  }
  if (clientComponents && typeof componentType === 'function') {
    const componentName = componentType.name || componentType.displayName
    if (componentName && clientComponents[componentName]) {
      return true
    }
  }
  if (componentType && typeof componentType === 'object' && Object.keys(componentType).length === 0) {
    return true
  }
  if (componentType && componentType.__isClientComponent) {
    return true
  }
  return false
}

function isServerComponent(componentType) {
  if (componentType && componentType.__isServerComponent) {
    return true
  }
  if (typeof componentType === 'function' && componentType.constructor.name === 'AsyncFunction') {
    return true
  }
  return false
}

function getClientComponentId(componentType, clientComponents) {
  if (componentType && typeof componentType === 'object') {
    const reactClientSymbol = Symbol.for('react.client.reference')
    if (componentType.$typeof === reactClientSymbol) {
      return componentType.$id
    }
  }
  if (clientComponents && typeof componentType === 'function') {
    const componentName = componentType.name || componentType.displayName
    if (componentName && clientComponents[componentName]) {
      return clientComponents[componentName].id || componentName
    }
  }
  return null
}

function isSuspenseComponent(type) {
  if (typeof React !== 'undefined' && React.Suspense && type === React.Suspense) {
    return true
  }
  if (type && type.$typeof === Symbol.for('react.suspense')) {
    return true
  }
  if (typeof type === 'function' && (type.name === 'Suspense' || type.displayName === 'Suspense')) {
    return true
  }
  if (type === 'suspense') {
    return true
  }
  return false
}

if (typeof globalThis !== 'undefined') {
  globalThis.traverseToRSCProfiled = traverseToRSCProfiled
  globalThis.resetRSCPerfStats = resetPerfStats
  globalThis.getRSCPerfStats = getPerfStats
}
