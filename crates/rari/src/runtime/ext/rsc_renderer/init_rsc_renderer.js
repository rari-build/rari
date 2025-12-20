import 'ext:init_rsc_renderer/render_element_to_html.js'
import 'ext:init_rsc_renderer/rsc_traversal.js'
import 'ext:init_rsc_renderer/client_registry.js'

if (typeof globalThis.renderElementToHtml === 'undefined') {
  throw new TypeError(
    'renderElementToHtml not initialized - render_element_to_html.js may have failed to load',
  )
}

if (typeof globalThis.renderToRsc === 'undefined') {
  throw new TypeError(
    'renderToRsc not initialized - rsc_traversal.js may have failed to load',
  )
}

if (typeof globalThis.registerClientComponent === 'undefined') {
  throw new TypeError(
    'registerClientComponent not initialized - client_registry.js may have failed to load',
  )
}

if (!globalThis['~rsc'])
  globalThis['~rsc'] = {}
if (!globalThis['~rsc'].rendererInitialized) {
  globalThis['~rsc'].rendererInitialized = true
  globalThis['~rsc'].rendererInitTimestamp = Date.now()

  globalThis['~rsc'].componentRenders
    = globalThis['~rsc'].componentRenders || new Map()
  globalThis['~rsc'].renderStatistics = globalThis['~rsc'].renderStatistics || {
    totalRenders: 0,
    successfulRenders: 0,
    failedRenders: 0,
    suspenseResolves: 0,
    rscRenders: 0,
  }

  globalThis['~track_component_render'] = function (componentName) {
    if (!globalThis['~rsc'].componentRenders.has(componentName)) {
      globalThis['~rsc'].componentRenders.set(componentName, 0)
    }
    globalThis['~rsc'].componentRenders.set(
      componentName,
      globalThis['~rsc'].componentRenders.get(componentName) + 1,
    )
    globalThis['~rsc'].renderStatistics.totalRenders++
  }

  globalThis['~track_rsc_render'] = function (componentName) {
    globalThis['~rsc'].renderStatistics.rscRenders++
    globalThis['~track_component_render'](componentName)
  }

  if (typeof globalThis['~clientComponents'] === 'undefined') {
    globalThis['~clientComponents'] = {}
  }
  if (typeof globalThis['~clientComponentNames'] === 'undefined') {
    globalThis['~clientComponentNames'] = {}
  }
  if (typeof globalThis['~clientComponentPaths'] === 'undefined') {
    globalThis['~clientComponentPaths'] = {}
  }
}
