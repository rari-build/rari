/* eslint-disable no-undef, style/object-curly-spacing */
// oxlint-disable @typescript-eslint/no-floating-promises
(async function () {
  const REACT_ELEMENT_TYPE = Symbol.for('react.transitional.element')
  const REACT_SUSPENSE_PENDING = Symbol.for('react.suspense.pending')

  let Component
  let componentSource = 'not found'

  if (typeof globalThis['{component_id}'] === 'function') {
    Component = globalThis['{component_id}']
    componentSource = 'global.{component_id}'
  }
  else if (
    globalThis['~rsc'].modules
    && globalThis['~rsc'].modules['{component_id}']
  ) {
    Component
      = globalThis['~rsc'].modules['{component_id}'].default
        || Object.values(globalThis['~rsc'].modules['{component_id}'])[0]
    componentSource = '~rsc.modules.{component_id}'
  }
  else {
    throw new Error('Component {component_id} not found in global scope')
  }

  const sanitizeComponentOutput = (html, componentId) => {
    if (typeof html !== 'string')
      return html

    return Deno.core.ops.op_sanitize_html(html, componentId)
  }

  const elementToRSC = async (element, componentId) => {
    try {
      const clientComponents = globalThis['~clientComponents'] || {}

      let rscResult
      if (typeof globalThis.renderToRsc === 'function') {
        const currentBoundaryId = globalThis['~suspense']?.currentBoundaryId || null
        rscResult = await globalThis.renderToRsc(element, clientComponents, currentBoundaryId)
      }
      else if (typeof globalThis.traverseToRsc === 'function') {
        rscResult = await globalThis.traverseToRsc(element, clientComponents)
      }
      else {
        rscResult = {
          $$typeof: REACT_ELEMENT_TYPE,
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
        $$typeof: REACT_ELEMENT_TYPE,
        type: 'div',
        props: {
          'data-rsc-component': componentId,
          'children': `Error: ${error.message}`,
        },
      }
    }
  }

  const props = {props_json}

  const isAsyncComponent = Component.constructor.name === 'AsyncFunction'

  let element
  if (isAsyncComponent) {
    try {
      const result = await Component(props)
      element = result
    }
    catch (asyncError) {
      console.error(`[rari] Error rendering ${componentSource}:`, asyncError)
      const errorResult = {
        html: '',
        rsc: null,
        hasSuspense: false,
        debug: {
          component_id: componentSource,
          success: false,
          error: asyncError.message,
        },
      }
      if (!globalThis['~render'])
        globalThis['~render'] = {}
      globalThis['~render'].lastResult = errorResult

      return errorResult
    }
  }
  else {
    element = Component(props)
  }

  try {
    const rscResult = await elementToRSC(element, '{component_id}')

    let htmlResult = null
    try {
      htmlResult = await renderToHtml(element)
      htmlResult = sanitizeComponentOutput(htmlResult, '{component_id}')
    }
    catch (htmlError) {
      console.warn('HTML generation failed, using RSC only:', htmlError)
      htmlResult = `<div data-rsc-component="{component_id}">RSC Component</div>`
    }

    if (!rscResult) {
      const emptyResult = {
        html: htmlResult || '',
        rsc: null,
        hasSuspense: false,
        debug: {
          component_id: componentSource,
          success: false,
          reason: 'empty_rsc',
        },
      }
      if (!globalThis['~render'])
        globalThis['~render'] = {}
      globalThis['~render'].lastResult = emptyResult

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

    if (!globalThis['~render'])
      globalThis['~render'] = {}
    globalThis['~render'].lastResult = finalResult

    return finalResult
  }
  catch (error) {
    if (error && error.$$typeof === REACT_SUSPENSE_PENDING) {
      if (error.promise && typeof error.promise.then === 'function') {
        try {
          await error.promise

          const newElement = isAsyncComponent ? await Component(props) : Component(props)

          const rscResult = await elementToRSC(newElement, '{component_id}')

          let htmlResult = null
          try {
            htmlResult = await renderToHtml(newElement)
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

          if (!globalThis['~render'])
            globalThis['~render'] = {}
          globalThis['~render'].lastResult = suspenseResolvedResult
          return suspenseResolvedResult
        }
        catch (resolveError) {
          console.error(`[rari] Error rendering ${componentSource} after suspense:`, resolveError)
          const errorResult = {
            html: '',
            rsc: null,
            hasSuspense: false,
            debug: {
              component_id: componentSource,
              success: false,
              error: resolveError.message,
            },
          }

          if (!globalThis['~render'])
            globalThis['~render'] = {}
          globalThis['~render'].lastResult = errorResult
          return errorResult
        }
      }
    }

    console.error(`[rari] Error rendering ${componentSource}:`, error)
    const errorResult = {
      html: '',
      rsc: null,
      hasSuspense: false,
      debug: {
        component_id: componentSource,
        success: false,
        error: error.message,
      },
    }

    if (!globalThis['~render'])
      globalThis['~render'] = {}
    globalThis['~render'].lastResult = errorResult

    return errorResult
  }
})()
