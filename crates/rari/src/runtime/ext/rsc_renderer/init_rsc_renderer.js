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

if (!globalThis.__rsc_renderer_initialized) {
  globalThis.__rsc_renderer_initialized = true
  globalThis.__rsc_renderer_init_timestamp = Date.now()

  globalThis.__rsc_component_renders
    = globalThis.__rsc_component_renders || new Map()
  globalThis.__rsc_render_statistics = globalThis.__rsc_render_statistics || {
    totalRenders: 0,
    successfulRenders: 0,
    failedRenders: 0,
    suspenseResolves: 0,
    rscRenders: 0,
  }

  globalThis.__track_component_render = function (componentName) {
    if (!globalThis.__rsc_component_renders.has(componentName)) {
      globalThis.__rsc_component_renders.set(componentName, 0)
    }
    globalThis.__rsc_component_renders.set(
      componentName,
      globalThis.__rsc_component_renders.get(componentName) + 1,
    )
    globalThis.__rsc_render_statistics.totalRenders++
  }

  globalThis.__track_rsc_render = function (componentName) {
    globalThis.__rsc_render_statistics.rscRenders++
    globalThis.__track_component_render(componentName)
  }

  if (typeof globalThis.__clientComponents === 'undefined') {
    globalThis.__clientComponents = {}
  }
  if (typeof globalThis.__clientComponentNames === 'undefined') {
    globalThis.__clientComponentNames = {}
  }
  if (typeof globalThis.__clientComponentPaths === 'undefined') {
    globalThis.__clientComponentPaths = {}
  }
}
