/* eslint-disable no-undef */
/* eslint-disable style/object-curly-spacing */
(async function () {
  let Component
  let componentSource = 'not found'

  if (typeof globalThis['{component_id}'] === 'function') {
    Component = globalThis['{component_id}']
    componentSource = 'global.{component_id}'
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

  const elementToRSC = (element, componentId) => {
    try {
      const clientComponents = globalThis.__rsc_client_components || {}

      let rscResult
      if (typeof globalThis.renderToRsc === 'function') {
        rscResult = globalThis.renderToRsc(element, clientComponents)
      }
      else if (typeof globalThis.traverseToRsc === 'function') {
        rscResult = globalThis.traverseToRsc(element, clientComponents)
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
      const errorResult = {
        html: `<div><h2>Error Rendering {component_id}</h2><p>${asyncError.message}</p></div>`,
        rsc: null,
        hasSuspense: false,
        debug: {
          component_id: componentSource,
          success: false,
          error: asyncError.message,
        },
      }
      globalThis.__lastRenderResult = errorResult

      return errorResult
    }
  }
  else {
    element = Component(props)
  }

  try {
    const rscResult = elementToRSC(element, '{component_id}')

    let htmlResult = null
    try {
      htmlResult = renderToHTML(element)
      htmlResult = sanitizeComponentOutput(htmlResult, '{component_id}')
    }
    catch (htmlError) {
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
          await error.promise

          const newElement = Component(props)

          const rscResult = elementToRSC(newElement, '{component_id}')

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

          const errorResult = {
            html: `<div><h2>Error Rendering {component_id}</h2><p>${finalError.message}</p></div>`,
            rsc: null,
            hasSuspense: false,
            debug: {
              component_id: componentSource,
              success: false,
              error: finalError.message,
            },
          }

          globalThis.__lastRenderResult = errorResult
          return errorResult
        }
      }
    }

    const errorResult = {
      html: `<div><h2>Error Rendering {component_id}</h2><p>${error.message}</p></div>`,
      rsc: null,
      hasSuspense: false,
      debug: {
        component_id: componentSource,
        success: false,
        error: error.message,
      },
    }

    globalThis.__lastRenderResult = errorResult

    return errorResult
  }
})()
