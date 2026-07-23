/// <reference path="../../types.d.ts" />

interface SuspenseError {
  $$typeof: symbol
  promise: Promise<unknown>
}

void (async function () {
  const REACT_SUSPENSE_PENDING = Symbol.for('react.suspense.pending')

  let Component: (props: unknown) => unknown
  let componentSource = 'not found'

  if (typeof g['{component_id}'] === 'function') {
    Component = g['{component_id}'] as (props: unknown) => unknown // oxlint-disable-line typescript/no-unsafe-type-assertion -- generated component id
    componentSource = 'global.{component_id}'
  } else if (g['~rsc']?.modules?.['{component_id}'] != null) {
    const moduleExports = g['~rsc'].modules['{component_id}']
    const resolved = moduleExports.default ?? Object.values(moduleExports)[0]
    if (typeof resolved !== 'function')
      throw new Error('Component {component_id} export is not a function')
    Component = resolved as (props: unknown) => unknown // oxlint-disable-line typescript/no-unsafe-type-assertion -- RSC module default export
    componentSource = '~rsc.modules.{component_id}'
  } else {
    throw new Error('Component {component_id} not found in global scope')
  }

  const sanitizeComponentOutput = (html: unknown, componentId: string): unknown => {
    if (typeof html !== 'string') return html

    return Deno.core.ops.op_sanitize_html(html, componentId)
  }

  const isSuspensePending = (error: unknown): error is SuspenseError => {
    return (
      error != null &&
      typeof error === 'object' &&
      '$$typeof' in error &&
      error.$$typeof === REACT_SUSPENSE_PENDING
    )
  }

  const elementToHtml = async (element: unknown, componentId: string): Promise<string | null> => {
    try {
      if (!g.renderToHtmlFizz) return null

      const htmlResult = await g.renderToHtmlFizz(element)
      const sanitized = sanitizeComponentOutput(htmlResult, componentId)
      if (typeof sanitized !== 'string' || sanitized.length === 0) return null

      return sanitized
    } catch (htmlError) {
      if (isSuspensePending(htmlError)) throw htmlError

      console.warn('HTML generation failed:', htmlError)
      return null
    }
  }

  const storeResult = (result: Readonly<Record<string, unknown>>) => {
    g['~render'] ??= {}
    g['~render'].lastResult = result
    return result
  }

  const renderOutputs = async (
    element: unknown,
    options?: Readonly<{ resolvedFromSuspense?: boolean }>,
  ) => {
    const htmlResult = await elementToHtml(element, '{component_id}')

    if (htmlResult == null) {
      return storeResult({
        html: '',
        hasSuspense: false,
        debug: {
          component_id: componentSource,
          success: false,
          reason: 'empty_html',
        },
      })
    }

    return storeResult({
      html: htmlResult,
      hasSuspense: options?.resolvedFromSuspense ?? false,
      debug: {
        component_id: componentSource,
        success: true,
        resolvedFromSuspense: options?.resolvedFromSuspense ?? false,
        htmlLength: htmlResult.length,
      },
    })
  }

  const props = { props_json }

  const isAsyncComponent = Component.constructor.name === 'AsyncFunction'

  let element: unknown
  if (isAsyncComponent) {
    try {
      const result = await Component(props)
      element = result
    } catch (asyncError: unknown) {
      const errorMessage = asyncError instanceof Error ? asyncError.message : String(asyncError)
      console.error(`[rari] Error rendering ${componentSource}:`, asyncError)
      return storeResult({
        html: '',
        hasSuspense: false,
        debug: {
          component_id: componentSource,
          success: false,
          error: errorMessage,
        },
      })
    }
  } else {
    element = Component(props)
  }

  try {
    return await renderOutputs(element)
  } catch (error: unknown) {
    if (isSuspensePending(error)) {
      if (typeof error.promise.then === 'function') {
        try {
          await error.promise
          const newElement = isAsyncComponent ? await Component(props) : Component(props)
          return await renderOutputs(newElement, { resolvedFromSuspense: true })
        } catch (resolveError: unknown) {
          const errorMessage =
            resolveError instanceof Error ? resolveError.message : String(resolveError)
          console.error(`[rari] Error rendering ${componentSource} after suspense:`, resolveError)
          return storeResult({
            html: '',
            hasSuspense: false,
            debug: {
              component_id: componentSource,
              success: false,
              error: errorMessage,
            },
          })
        }
      }
    }

    const errorMessage = error instanceof Error ? error.message : String(error)
    console.error(`[rari] Error rendering ${componentSource}:`, error)
    return storeResult({
      html: '',
      hasSuspense: false,
      debug: {
        component_id: componentSource,
        success: false,
        error: errorMessage,
      },
    })
  }
})()
