/* eslint-disable no-undef */
(async () => {
  const layoutPaths = LAYOUT_PATHS_PLACEHOLDER
  const pagePath = 'PAGE_PATH_PLACEHOLDER'
  const params = PARAMS_PLACEHOLDER
  const searchParams = SEARCH_PARAMS_PLACEHOLDER

  let metadata = {}

  function mergeMetadata(parent, child) {
    const merged = { ...parent }

    if (child.title !== undefined) {
      if (typeof child.title === 'string') {
        if (typeof parent.title === 'object' && parent.title?.template)
          merged.title = parent.title.template.replace('%s', child.title)
        else
          merged.title = child.title
      }
      else {
        merged.title = child.title
      }
    }

    if (child.description !== undefined)
      merged.description = child.description
    if (child.keywords !== undefined)
      merged.keywords = child.keywords
    if (child.openGraph !== undefined)
      merged.openGraph = { ...parent.openGraph, ...child.openGraph }
    if (child.twitter !== undefined)
      merged.twitter = { ...parent.twitter, ...child.twitter }
    if (child.robots !== undefined)
      merged.robots = { ...parent.robots, ...child.robots }
    if (child.icons !== undefined)
      merged.icons = { ...parent.icons, ...child.icons }
    if (child.manifest !== undefined)
      merged.manifest = child.manifest
    if (child.themeColor !== undefined)
      merged.themeColor = child.themeColor
    if (child.appleWebApp !== undefined)
      merged.appleWebApp = { ...parent.appleWebApp, ...child.appleWebApp }
    if (child.viewport !== undefined)
      merged.viewport = child.viewport
    if (child.canonical !== undefined)
      merged.canonical = child.canonical

    return merged
  }

  async function extractMetadata(modulePath, params, searchParams) {
    try {
      if (typeof globalThis['~rsc'].modules !== 'undefined') {
        const moduleKey = modulePath.replace(/^file:\/\/.*\/app\//, 'app/').replace(/\.js$/, '')
        const module = globalThis['~rsc'].modules[moduleKey]

        if (module) {
          if (typeof module.generateMetadata === 'function') {
            const result = await module.generateMetadata({ params, searchParams })
            if (result && typeof result === 'object')
              return result
          }

          if (module.metadata && typeof module.metadata === 'object')
            return module.metadata
        }
      }

      const module = await import(modulePath)

      if (typeof module.generateMetadata === 'function') {
        const result = await module.generateMetadata({ params, searchParams })
        if (result && typeof result === 'object')
          return result
      }

      if (module.metadata && typeof module.metadata === 'object')
        return module.metadata

      return {}
    }
    catch (error) {
      console.error(`Failed to extract metadata from ${modulePath}:`, error)
      return {}
    }
  }

  for (const layoutPath of layoutPaths) {
    const layoutMetadata = await extractMetadata(layoutPath, params, searchParams)
    metadata = mergeMetadata(metadata, layoutMetadata)
  }

  const pageMetadata = await extractMetadata(pagePath, params, searchParams)
  metadata = mergeMetadata(metadata, pageMetadata)

  if (metadata.title && typeof metadata.title === 'object') {
    if (metadata.title.absolute)
      metadata.title = metadata.title.absolute
    else if (metadata.title.default)
      metadata.title = metadata.title.default
  }

  return metadata
})()
