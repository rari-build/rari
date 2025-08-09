/* eslint-disable no-undef */
/* eslint-disable style/object-curly-spacing */

(async function () {
  let Component
  let componentSource = 'not found'

  globalThis.__current_component_id = '{component_id}'

  if (typeof globalThis['{component_id}'] === 'function') {
    Component = globalThis['{component_id}']
    componentSource = 'global.{component_id}'
  }
  else if (typeof globalThis['Component_{component_hash}'] === 'function') {
    Component = globalThis['Component_{component_hash}']
    componentSource = 'global.Component_{component_hash}'
  }
  else if (
    globalThis.__rsc_modules
    && globalThis.__rsc_modules['{component_id}']
  ) {
    Component
      = globalThis.__rsc_modules['{component_id}'].default
        || Object.values(globalThis.__rsc_modules['{component_id}'])[0]
    componentSource = '__rsc_modules.{component_id}'
  }
  else {
    throw new Error('Component {component_id} not found in global scope')
  }

  const handleSuspensePromise = async (promise, _componentId, _boundaryId) => {
    try {
      const result = await promise

      if (promise.__cacheKey && globalThis.__promise_cache) {
        globalThis.__promise_cache.set(promise.__cacheKey, {
          resolved: true,
          value: result,
          resolvedAt: Date.now(),
        })
      }

      if (globalThis.__suspense_manager && promise.__promiseId) {
        globalThis.__suspense_manager.resolvePromise(
          promise.__promiseId,
          result,
        )
      }

      return result
    }
    catch (resolveError) {
      if (globalThis.__suspense_manager && promise.__promiseId) {
        globalThis.__suspense_manager.rejectPromise(
          promise.__promiseId,
          resolveError,
        )
      }
      throw resolveError
    }
  }

  const sanitizeComponentOutput = (html, componentId) => {
    if (typeof html !== 'string')
      return html

    const wrapperRegex = new RegExp(
      `<div[^>]*?data-component-id=["']${componentId}["'][^>]*?>([\\s\\S]*?)<\\/div>`,
      'i',
    )
    const match = html.match(wrapperRegex)

    if (match) {
      // Only focus on content specifically marked for this component
      // This helps prevent cross-component contamination
    }

    const jsonCleanupPatterns = [
      { pattern: /<pre>(\\\{[\\sS]*?\\\})<\/pre>/g, replacement: '' },
      { pattern: /\\\{"id":.*?\\\}/g, replacement: '' },
    ]

    for (const { pattern, replacement } of jsonCleanupPatterns) {
      html = html.replace(pattern, replacement)
    }

    return html
  }

  const elementToRSC = async (element, componentId) => {
    try {
      const clientComponents = globalThis.__rsc_client_components || {}

      let rscResult
      if (typeof globalThis.renderToRSC === 'function') {
        rscResult = await globalThis.renderToRSC(element, clientComponents)
      }
      else if (typeof globalThis.traverseToRSC === 'function') {
        rscResult = await globalThis.traverseToRSC(element, clientComponents)
      }
      else {
        rscResult = {
          $$typeof: Symbol.for('react.element'),
          type: 'div',
          props: {
            'data-rsc-component': componentId,
            'children': element,
          },
        }
      }

      return rscResult
    }
    catch (error) {
      return {
        $$typeof: Symbol.for('react.element'),
        type: 'div',
        props: {
          'data-rsc-component': componentId,
          'children': `Error: ${error.message}`,
        },
      }
    }
  }

  // prettier-ignore
  const props = {props_json}

  const isAsyncComponent = Component.constructor.name === 'AsyncFunction'

  let element
  if (isAsyncComponent) {
    try {
      const result = await Component(props)
      element = result
    }
    catch (asyncError) {
      if (
        asyncError
        && asyncError.$$typeof === Symbol.for('react.suspense.pending')
      ) {
        throw asyncError
      }

      const errorResult = {
        html: `<div><h2>Sync Error Rendering {component_id}</h2><p>${syncError.message}</p></div>`,
        rsc: null,
        hasSuspense: false,
        debug: {
          component_id: componentSource,
          success: false,
          error: syncError.message,
        },
      }
      globalThis.__lastRenderResult = errorResult
      return errorResult
    }
  }
  else {
    try {
      element = React.createElement(Component, props)
    }
    catch (syncError) {
      if (
        syncError
        && syncError.$$typeof === Symbol.for('react.suspense.pending')
      ) {
        throw syncError
      }
      throw syncError
    }
  }

  try {
    const rscResult = await elementToRSC(element, '{component_id}')

    let htmlResult = null
    try {
      htmlResult = renderToHTML(element)
      htmlResult = sanitizeComponentOutput(htmlResult, '{component_id}')
    }
    catch (htmlError) {
      if (
        htmlError
        && htmlError.$$typeof === Symbol.for('react.suspense.pending')
      ) {
        throw htmlError
      }
      console.warn('HTML generation failed, using RSC only:', htmlError)
      htmlResult = `<div data-rsc-component="{component_id}">RSC Component</div>`
    }

    if (!rscResult) {
      const emptyResult = {
        html:
          htmlResult
          || `<div><h2>Component: ${componentSource}</h2><p>Empty result from component rendering</p></div>`,
        rsc: null,
        hasSuspense: false,
        debug: {
          component_id: componentSource,
          success: false,
          reason: 'empty_rsc',
        },
      }

      globalThis.__lastRenderResult = emptyResult
      return emptyResult
    }

    const finalResult = {
      html: htmlResult,
      rsc: rscResult,
      hasSuspense: false,
      debug: {
        component_id: componentSource,
        success: true,
        htmlLength: htmlResult ? htmlResult.length : 0,
        hasRSC: !!rscResult,
      },
    }

    globalThis.__lastRenderResult = finalResult

    return finalResult
  }
  catch (error) {
    if (error && error.$$typeof === Symbol.for('react.suspense.pending')) {
      if (error.promise && typeof error.promise.then === 'function') {
        try {
          const resolvedValue = await handleSuspensePromise(
            error.promise,
            '{component_id}',
            error.__boundaryId,
          )

          let newElement
          if (isAsyncComponent) {
            newElement = await Component(props)
          }
          else {
            newElement = React.createElement(Component, props)
          }

          const rscResult = await elementToRSC(newElement, '{component_id}')

          let htmlResult = null
          try {
            htmlResult = renderToHTML(newElement)
            htmlResult = sanitizeComponentOutput(htmlResult, '{component_id}')
          }
          catch (htmlError) {
            console.warn(
              'HTML generation failed after suspense, using RSC only:',
              htmlError,
            )
            htmlResult = `<div data-rsc-component="{component_id}">RSC Component (Suspense Resolved)</div>`
          }

          const suspenseResolvedResult = {
            html: htmlResult,
            rsc: rscResult,
            hasSuspense: true,
            suspenseResolution: {
              boundaryId: error.__boundaryId,
              promiseId: error.__promiseId,
              cacheKey: error.__cacheKey,
              resolvedValue,
            },
            debug: {
              component_id: componentSource,
              success: true,
              resolvedFromSuspense: true,
              htmlLength: htmlResult ? htmlResult.length : 0,
              hasRSC: !!rscResult,
            },
          }

          globalThis.__lastRenderResult = suspenseResolvedResult
          return suspenseResolvedResult
        }
        catch (resolveError) {
          const finalError = resolveError

          const withinSuspenseBoundary
            = error.__boundaryId && globalThis.__suspense_manager
          if (withinSuspenseBoundary) {
            globalThis.__suspense_manager.rejectPromise(
              error.__promiseId,
              resolveError,
            )
          }

          const errorResult = {
            html: `<div><h2>Suspense Error in {component_id}</h2><p>${finalError.message}</p><details><summary>Stack</summary><pre>${finalError.stack || 'No stack available'}</pre></details></div>`,
            rsc: null,
            hasSuspense: true,
            suspenseError: {
              boundaryId: error.__boundaryId,
              promiseId: error.__promiseId,
              cacheKey: error.__cacheKey,
              error: finalError.message,
              stack: finalError.stack,
            },
            debug: {
              component_id: componentSource,
              success: false,
              error: finalError.message,
              suspenseFailure: true,
            },
          }

          globalThis.__lastRenderResult = errorResult
          return errorResult
        }
      }
      else {
        const invalidSuspenseError = {
          html: `<div><h2>Invalid Suspense Promise in {component_id}</h2><p>Suspense promise must have a .then method</p></div>`,
          rsc: null,
          hasSuspense: false,
          debug: {
            component_id: componentSource,
            success: false,
            error: 'Invalid Suspense promise: missing .then method',
          },
        }

        globalThis.__lastRenderResult = invalidSuspenseError
        return invalidSuspenseError
      }
    }

    const errorResult = {
      html: `<div><h2>Error Rendering {component_id}</h2><p>${error.message}</p><details><summary>Stack</summary><pre>${error.stack || 'No stack available'}</pre></details></div>`,
      rsc: null,
      hasSuspense: false,
      debug: {
        component_id: componentSource,
        success: false,
        error: error.message,
        stack: error.stack,
      },
    }

    globalThis.__lastRenderResult = errorResult

    return errorResult
  }
  finally {
    globalThis.__current_component_id = null
  }
})()
