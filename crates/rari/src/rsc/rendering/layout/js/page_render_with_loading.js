/* eslint-disable unused-imports/no-unused-vars, no-undef, style/object-curly-spacing */
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
  const isAsync = pageResult && typeof pageResult.then === 'function'
  var pageElement = pageResult
  timings.isAsync = isAsync
}
else {
  const pageProps = {page_props_json}
  const useSuspense = {use_suspense}

  const isAsync = PageComponent.constructor.name === 'AsyncFunction'

  const pageResult = PageComponent(pageProps)
  const isAsyncResult = pageResult && typeof pageResult.then === 'function'

  var pageElement = pageResult
  timings.isAsync = isAsync || isAsyncResult
}
timings.pageRender = performance.now() - startPage
