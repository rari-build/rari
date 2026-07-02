/// <reference path="../../types.d.ts" />

interface SuspenseError {
  $$typeof: symbol
  promise: Promise<unknown>
}

(async function () {
  const REACT_ELEMENT_TYPE = Symbol.for('react.transitional.element')
  const REACT_SUSPENSE_PENDING = Symbol.for('react.suspense.pending')

  let Component: (props: unknown) => unknown
  let componentSource = 'not found'

  if (typeof g['{component_id}'] === 'function') {
    Component = g['{component_id}'] as (props: unknown) => unknown
    componentSource = 'global.{component_id}'
  }
  else if (
    g['~rsc']?.modules
    && g['~rsc'].modules['{component_id}']
  ) {
    Component
      = (g['~rsc'].modules['{component_id}'].default
        || Object.values(g['~rsc'].modules['{component_id}'])[0]) as (props: unknown) => unknown
    componentSource = '~rsc.modules.{component_id}'
  }
  else {
    throw new Error('Component {component_id} not found in global scope')
  }

  const sanitizeComponentOutput = (html: unknown, componentId: string): unknown => {
    if (typeof html !== 'string')
      return html

    return Deno.core.ops.op_sanitize_html(html, componentId)
  }

  const elementToRSC = async (element: unknown, componentId: string): Promise<unknown> => {
    try {
      let rscResult: unknown
      if (typeof g.renderToRsc === 'function') {
        rscResult = await g.renderToRsc(element)
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
    catch (error: unknown) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      return {
        $$typeof: REACT_ELEMENT_TYPE,
        type: 'div',
        props: {
          'data-rsc-component': componentId,
          'children': `Error: ${errorMessage}`,
        },
      }
    }
  }

  const props = {props_json}

  const isAsyncComponent = Component.constructor.name === 'AsyncFunction'

  let element: unknown
  if (isAsyncComponent) {
    try {
      const result = await Component(props)
      element = result
    }
    catch (asyncError: unknown) {
      const errorMessage = asyncError instanceof Error ? asyncError.message : String(asyncError)
      console.error(`[rari] Error rendering ${componentSource}:`, asyncError)
      const errorResult = {
        html: '',
        rsc: null,
        hasSuspense: false,
        debug: {
          component_id: componentSource,
          success: false,
          error: errorMessage,
        },
      }

      if (!g['~render'])
        g['~render'] = {}
      g['~render'].lastResult = errorResult

      return errorResult
    }
  }
  else {
    element = Component(props)
  }

  try {
    const rscResult = await elementToRSC(element, '{component_id}')

    let htmlResult: string | null = null
    try {
      htmlResult = g.renderToHtmlFizz ? await g.renderToHtmlFizz(element) : null
      htmlResult = sanitizeComponentOutput(htmlResult, '{component_id}') as string
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

      if (!g['~render'])
        g['~render'] = {}
      g['~render'].lastResult = emptyResult

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

    if (!g['~render'])
      g['~render'] = {}
    g['~render'].lastResult = finalResult

    return finalResult
  }
  catch (error: unknown) {
    const suspenseError = error as SuspenseError
    if (suspenseError && suspenseError.$$typeof === REACT_SUSPENSE_PENDING) {
      if (suspenseError.promise && typeof suspenseError.promise.then === 'function') {
        try {
          await suspenseError.promise

          const newElement = isAsyncComponent ? await Component(props) : Component(props)

          const rscResult = await elementToRSC(newElement, '{component_id}')

          let htmlResult: string | null = null
          try {
            htmlResult = g.renderToHtmlFizz ? await g.renderToHtmlFizz(newElement) : null
            htmlResult = sanitizeComponentOutput(htmlResult, '{component_id}') as string
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

          if (!g['~render'])
            g['~render'] = {}
          g['~render'].lastResult = suspenseResolvedResult

          return suspenseResolvedResult
        }
        catch (resolveError: unknown) {
          const errorMessage = resolveError instanceof Error ? resolveError.message : String(resolveError)
          console.error(`[rari] Error rendering ${componentSource} after suspense:`, resolveError)
          const errorResult = {
            html: '',
            rsc: null,
            hasSuspense: false,
            debug: {
              component_id: componentSource,
              success: false,
              error: errorMessage,
            },
          }

          if (!g['~render'])
            g['~render'] = {}
          g['~render'].lastResult = errorResult

          return errorResult
        }
      }
    }

    const errorMessage = error instanceof Error ? error.message : String(error)
    console.error(`[rari] Error rendering ${componentSource}:`, error)
    const errorResult = {
      html: '',
      rsc: null,
      hasSuspense: false,
      debug: {
        component_id: componentSource,
        success: false,
        error: errorMessage,
      },
    }

    if (!g['~render'])
      g['~render'] = {}
    g['~render'].lastResult = errorResult

    return errorResult
  }
})()
