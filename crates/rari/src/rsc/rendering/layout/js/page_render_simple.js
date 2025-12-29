// oxlint-disable vars-on-top, no-var
/* eslint-disable unused-imports/no-unused-vars, no-undef, style/object-curly-spacing */
const startPage = performance.now()
const PageComponent = globalThis['{page_component_id}']
if (!PageComponent || typeof PageComponent !== 'function') {
  throw new Error('Page component {page_component_id} not found')
}

const pageProps = {page_props_json}
const pageElement = React.createElement(PageComponent, pageProps)
const isAsync = PageComponent.constructor.name === 'AsyncFunction'

timings.isAsync = isAsync
timings.pageRender = performance.now() - startPage
