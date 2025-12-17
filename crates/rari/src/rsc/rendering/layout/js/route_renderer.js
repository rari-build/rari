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

  const clientComponents = globalThis.__rsc_client_components || {}
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
        error: 'renderToHtml function not available',
        success: false,
      }
    }

    const html = await globalThis.renderToHtml(currentElement)

    return {
      html,
      error: null,
      success: true,
    }
  }
  catch (error) {
    console.error('Error in renderRouteToHtml:', error)
    return {
      html: '',
      error: error.message || String(error),
      success: false,
    }
  }
}
