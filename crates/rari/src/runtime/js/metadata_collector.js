/* eslint-disable no-undef */
(async () => {
  const layoutPaths = LAYOUT_PATHS_PLACEHOLDER
  const pagePath = 'PAGE_PATH_PLACEHOLDER'
  const params = PARAMS_PLACEHOLDER
  const searchParams = SEARCH_PARAMS_PLACEHOLDER

  const metadataList = []

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
    metadataList.push(layoutMetadata)
  }

  const pageMetadata = await extractMetadata(pagePath, params, searchParams)
  metadataList.push(pageMetadata)

  return metadataList
})()
