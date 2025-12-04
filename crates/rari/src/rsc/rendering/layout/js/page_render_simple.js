// oxlint-disable vars-on-top, no-var
/* eslint-disable unused-imports/no-unused-vars, antfu/no-top-level-await, no-undef, style/object-curly-spacing */
const startPage = performance.now()
const PageComponent = globalThis['{page_component_id}']
if (!PageComponent || typeof PageComponent !== 'function') {
  throw new Error('Page component {page_component_id} not found')
}

const pageProps = {page_props_json}
const pageResult = PageComponent(pageProps)
var pageElement = pageResult && typeof pageResult.then === 'function'
  ? await pageResult
  : pageResult
timings.pageRender = performance.now() - startPage
