/* eslint-disable unused-imports/no-unused-vars, antfu/no-top-level-await, no-undef, style/object-curly-spacing */
// oxlint-disable vars-on-top, no-var, block-scoped-var
const startPage = performance.now()
const PageComponent = globalThis['{page_component_id}']
if (!PageComponent || typeof PageComponent !== 'function') {
  throw new Error('Page component {page_component_id} not found')
}

const LoadingComponent = globalThis['{loading_id}']
if (!LoadingComponent || typeof LoadingComponent !== 'function') {
  console.warn('Loading component {loading_id} not found, rendering page without Suspense')
  const pageProps = {page_props_json}
  const pageResult = PageComponent(pageProps)
  var pageElement = pageResult && typeof pageResult.then === 'function'
    ? await pageResult
    : pageResult
}
else {
  const pageProps = {page_props_json}
  const useSuspense = {use_suspense}

  const isAsync = PageComponent.constructor.name === 'AsyncFunction'

  if (isAsync && useSuspense) {
    try {
      const componentPathHash = '{route_file_path}'
      const boundaryId = `page_boundary_${componentPathHash}`
      const promiseId = `page_promise_${componentPathHash}`

      if (!globalThis['~suspense'])
        globalThis['~suspense'] = {}
      if (!globalThis['~suspense'].promises)
        globalThis['~suspense'].promises = {}

      if (!globalThis['~render'])
        globalThis['~render'] = {}
      if (!globalThis['~render'].deferredAsyncComponents)
        globalThis['~render'].deferredAsyncComponents = []
      globalThis['~render'].deferredAsyncComponents.push({
        promiseId,
        boundaryId,
        component: PageComponent,
        props: pageProps,
        componentPath: '{route_file_path}',
      })

      if (!globalThis['~suspense'].discoveredBoundaries)
        globalThis['~suspense'].discoveredBoundaries = []

      if (!globalThis['~suspense'].pendingPromises)
        globalThis['~suspense'].pendingPromises = []
      globalThis['~suspense'].pendingPromises.push({
        id: promiseId,
        boundaryId,
        componentPath: '{route_file_path}',
      })

      let loadingFallback
      try {
        loadingFallback = LoadingComponent()
      }
      catch (loadingError) {
        throw new Error(`Failed to call LoadingComponent: ${loadingError.message || String(loadingError)}`)
      }

      const fallbackForBoundary = {
        type: loadingFallback?.type || 'div',
        props: loadingFallback?.props ? { ...loadingFallback.props } : { children: 'Loading...' },
        key: null,
      }

      globalThis['~suspense'].discoveredBoundaries.push({
        id: boundaryId,
        fallback: fallbackForBoundary,
        parentId: 'content-slot',
        parentPath: ['content-slot'],
        isInContentArea: true,
        positionHints: {
          inContentArea: true,
          domPath: ['content-slot'],
          isStable: true,
        },
      })

      const childrenPlaceholder = React.createElement('div', {
        'data-~promise-ref': promiseId,
        'className': 'suspense-placeholder',
      }, 'Loading...')
      let suspenseRscProps
      try {
        const fallbackRsc = await globalThis.renderToRsc(loadingFallback, globalThis['~rsc'].clientComponents || {})

        suspenseRscProps = {
          'fallback': fallbackRsc,
          '~boundaryId': boundaryId,
        }
      }
      catch (renderError) {
        throw new Error(`Failed to render Suspense boundary: ${renderError.message || String(renderError)}`)
      }

      var pageElement = {
        '~preSerializedSuspense': true,
        'rscArray': ['$', 'react.suspense', null, suspenseRscProps],
      }
    }
    catch (asyncWrapError) {
      throw new Error(`Failed to wrap async component in Suspense: ${asyncWrapError.message || String(asyncWrapError)}`)
    }
  }
  else if (isAsync && !useSuspense) {
    try {
      const pageResult = PageComponent(pageProps)

      if (pageResult && typeof pageResult.then === 'function') {
        var pageElement = await pageResult
      }
      else {
        var pageElement = pageResult
      }
    }
    catch (asyncError) {
      throw new Error(`Failed to await async component: ${asyncError.message || String(asyncError)}`)
    }
  }
  else {
    const pageResult = PageComponent(pageProps)

    var pageElement = pageResult && typeof pageResult.then === 'function'
      ? await pageResult
      : pageResult
  }
}
timings.pageRender = performance.now() - startPage
