/* eslint-disable unused-imports/no-unused-vars, no-undef, style/object-curly-spacing */
// oxlint-disable vars-on-top, no-var, block-scoped-var, no-redeclare
const startPage = performance.now()
const PageComponent = globalThis['{page_component_id}']
if (!PageComponent || typeof PageComponent !== 'function')
  throw new Error('Page component {page_component_id} not found')

const LoadingComponent = globalThis['{loading_id}']
if (!LoadingComponent || typeof LoadingComponent !== 'function') {
  const pageProps = {page_props_json}
  var pageElement = React.createElement(PageComponent, pageProps)
  timings.isAsync = PageComponent.constructor.name === 'AsyncFunction'
}
else {
  const pageProps = {page_props_json}
  const useSuspense = {use_suspense}

  const isAsync = PageComponent.constructor.name === 'AsyncFunction'

  if (isAsync && useSuspense) {
    const streamingEnabled = globalThis.__RARI_STREAMING_SUSPENSE__ === true

    if (streamingEnabled) {
      if (!globalThis.__RARI_PENDING_PROMISES__)
        globalThis.__RARI_PENDING_PROMISES__ = new Map()

      const promiseId = '{page_component_id}_promise'

      if (!globalThis.__RARI_PENDING_PROMISES__.has(promiseId)) {
        globalThis.__RARI_PENDING_PROMISES__.set(promiseId, {
          component: PageComponent,
          props: pageProps,
          isDeferred: true,
        })
      }

      const lazyMarker = {
        __rari_lazy: true,
        __rari_promise_id: promiseId,
        __rari_component_id: '{route_file_path}#default',
        __rari_loading_id: '{loading_file_path}#default',
      }

      const loadingFallback = LoadingComponent()
      var pageElement = React.createElement(
        React.Suspense,
        { 'fallback': loadingFallback, '~boundaryId': promiseId },
        lazyMarker,
      )
    }
    else {
      const pageResult = (async () => await PageComponent(pageProps))()

      const loadingFallback = LoadingComponent()
      var pageElement = React.createElement(
        React.Suspense,
        { fallback: loadingFallback },
        pageResult,
      )
    }
  }
  else {
    var pageElement = React.createElement(PageComponent, pageProps)
  }

  timings.isAsync = isAsync
}
timings.pageRender = performance.now() - startPage
