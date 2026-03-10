/* eslint-disable unused-imports/no-unused-vars, no-undef, style/object-curly-spacing */
// oxlint-disable vars-on-top, no-var, block-scoped-var, no-redeclare
const startPage = performance.now()
const PageComponent = globalThis['{page_component_id}']
if (!PageComponent || typeof PageComponent !== 'function')
  throw new Error('Page component {page_component_id} not found')

let pageElement
const LoadingComponent = globalThis['{loading_id}']
if (!LoadingComponent || typeof LoadingComponent !== 'function') {
  const pageProps = {page_props_json}
  pageElement = React.createElement(PageComponent, pageProps)
  timings.isAsync = PageComponent.constructor.name === 'AsyncFunction'
}
else {
  const pageProps = {page_props_json}
  const useSuspense = {use_suspense}

  const isAsync = PageComponent.constructor.name === 'AsyncFunction'

  if (isAsync && useSuspense) {
    const streamingEnabled = globalThis['~RARI_STREAMING_SUSPENSE'] === true

    if (streamingEnabled) {
      if (!globalThis['~RARI_PENDING_PROMISES'])
        globalThis['~RARI_PENDING_PROMISES'] = new Map()

      if (!globalThis['~RARI_PROMISE_COUNTER'])
        globalThis['~RARI_PROMISE_COUNTER'] = 0
      globalThis['~RARI_PROMISE_COUNTER']++

      const promiseId = `{page_component_id}_promise_${globalThis['~RARI_PROMISE_COUNTER']}`

      globalThis['~RARI_PENDING_PROMISES'].set(promiseId, {
        component: PageComponent,
        props: pageProps,
        isDeferred: true,
      })

      const lazyMarker = {
        __rari_lazy: true,
        __rari_promise_id: promiseId,
        __rari_component_id: '{route_file_path}#default',
        __rari_loading_id: '{loading_file_path}#default',
      }

      const loadingFallback = LoadingComponent()

      pageElement = React.createElement(
        React.Suspense,
        { 'fallback': loadingFallback, '~boundaryId': promiseId },
        lazyMarker,
      )
    }
    else {
      const pageResult = (async () => await PageComponent(pageProps))()

      const loadingFallback = LoadingComponent()
      pageElement = React.createElement(
        React.Suspense,
        { fallback: loadingFallback },
        pageResult,
      )
    }
  }
  else {
    pageElement = React.createElement(PageComponent, pageProps)
  }

  timings.isAsync = isAsync
}
timings.pageRender = performance.now() - startPage
