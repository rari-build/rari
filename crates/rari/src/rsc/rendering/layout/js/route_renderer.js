/* eslint-disable no-undef */
globalThis.renderRoute = async function (pageComponentId, pageProps, layouts) {
  const PageComponent = globalThis[pageComponentId]
  if (!PageComponent || typeof PageComponent !== 'function') {
    throw new TypeError(`Page component ${pageComponentId} not found`)
  }

  const pageResult = PageComponent(pageProps)
  let currentElement = pageResult && typeof pageResult.then === 'function'
    ? await pageResult
    : pageResult

  for (let i = layouts.length - 1; i >= 0; i--) {
    const layout = layouts[i]
    const LayoutComponent = globalThis[layout.componentId]

    if (!LayoutComponent || typeof LayoutComponent !== 'function') {
      throw new TypeError(`Layout component ${layout.componentId} not found`)
    }

    const layoutResult = LayoutComponent({ children: currentElement })
    currentElement = layoutResult && typeof layoutResult.then === 'function'
      ? await layoutResult
      : layoutResult
  }

  const clientComponents = globalThis['~rsc'].clientComponents || {}
  let rscResult

  if (typeof globalThis.renderToRsc === 'function') {
    rscResult = globalThis.renderToRsc(currentElement, clientComponents)
  }
  else if (typeof globalThis.traverseToRsc === 'function') {
    rscResult = globalThis.traverseToRsc(currentElement, clientComponents)
  }
  else {
    throw new TypeError('No RSC renderer available (renderToRsc or traverseToRsc)')
  }

  return {
    rsc: rscResult,
    success: true,
  }
}

globalThis.renderRouteToHtml = async function (pageComponentId, pageProps, layouts) {
  const PageComponent = globalThis[pageComponentId]
  if (!PageComponent || typeof PageComponent !== 'function') {
    return {
      html: '',
      rsc: '',
      error: `Page component ${pageComponentId} not found`,
      success: false,
    }
  }

  try {
    const pageResult = PageComponent(pageProps)
    let currentElement = pageResult && typeof pageResult.then === 'function'
      ? await pageResult
      : pageResult

    for (let i = layouts.length - 1; i >= 0; i--) {
      const layout = layouts[i]
      const LayoutComponent = globalThis[layout.componentId]

      if (!LayoutComponent || typeof LayoutComponent !== 'function') {
        return {
          html: '',
          rsc: '',
          error: `Layout component ${layout.componentId} not found`,
          success: false,
        }
      }

      const layoutResult = LayoutComponent({ children: currentElement })
      currentElement = layoutResult && typeof layoutResult.then === 'function'
        ? await layoutResult
        : layoutResult
    }

    if (typeof globalThis.renderToHtml !== 'function') {
      return {
        html: '',
        rsc: '',
        error: 'renderToHtml function not available',
        success: false,
      }
    }

    const AppRouterProviderShell = ({ children }) => React.createElement(React.Fragment, {}, children)
    const ClientRouterShell = ({ children }) => React.createElement(React.Fragment, {}, children)

    let htmlElement = React.createElement(
      AppRouterProviderShell,
      {},
      currentElement,
    )

    htmlElement = React.createElement(
      ClientRouterShell,
      { initialRoute: pageProps.pathname || '/' },
      htmlElement,
    )

    const html = await globalThis.renderToHtml(htmlElement)

    let rscData = null
    try {
      const clientComponents = globalThis['~rsc']?.clientComponents || {}

      if (typeof globalThis.renderToRsc === 'function') {
        rscData = await globalThis.renderToRsc(currentElement, clientComponents)
      }
      else if (typeof globalThis.traverseToRsc === 'function') {
        rscData = await globalThis.traverseToRsc(currentElement, clientComponents)
      }
    }
    catch (rscError) {
      console.error('[renderRouteToHtml] Failed to generate RSC payload:', rscError)
    }

    return {
      html,
      rscData,
      error: null,
      success: true,
    }
  }
  catch (error) {
    console.error('Error in renderRouteToHtml:', error)
    return {
      html: '',
      rsc: '',
      error: error.message || String(error),
      success: false,
    }
  }
}
