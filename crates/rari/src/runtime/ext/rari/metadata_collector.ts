/* oxlint-disable no-undef */
/// <reference path="./types.d.ts" />

interface MetadataParams {
  params: Record<string, string>
  searchParams: Record<string, string>
}

interface PageMetadata {
  title?: string
  description?: string
  [key: string]: unknown
}

type GenerateMetadataFn = (props: MetadataParams) => Promise<PageMetadata> | PageMetadata

interface ModuleWithMetadata {
  metadata?: PageMetadata
  generateMetadata?: GenerateMetadataFn
}

const FILE_URL_REGEX = /^file:\/\/.*\/app\//
const JS_EXTENSION_REGEX = /\.js$/

async function collect(
  layoutPaths: string[],
  pagePath: string,
  params: Record<string, string>,
  searchParams: Record<string, string>,
): Promise<PageMetadata[]> {
  const metadataList: PageMetadata[] = []

  async function extractMetadata(
    modulePath: string,
    params: Record<string, string>,
    searchParams: Record<string, string>,
  ): Promise<PageMetadata> {
    try {
      // @ts-expect-error - ~rsc is dynamically added to globalThis
      if (globalThis['~rsc'] && globalThis['~rsc'].modules !== undefined) {
        const moduleKey = modulePath
          .replace(FILE_URL_REGEX, 'app/')
          .replace(JS_EXTENSION_REGEX, '')

        // @ts-expect-error - ~rsc is dynamically added to globalThis
        const module = globalThis['~rsc'].modules[moduleKey] as ModuleWithMetadata | undefined

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

      const module = await import(modulePath) as ModuleWithMetadata

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

  const validLayoutPaths = Array.isArray(layoutPaths) ? layoutPaths : []
  for (const layoutPath of validLayoutPaths) {
    const layoutMetadata = await extractMetadata(layoutPath, params, searchParams)
    metadataList.push(layoutMetadata)
  }

  const pageMetadata = await extractMetadata(pagePath, params, searchParams)
  metadataList.push(pageMetadata)

  return metadataList
}

// @ts-expect-error - ~rari is dynamically added to globalThis
if (!globalThis['~rari'])
  // @ts-expect-error - ~rari is dynamically added to globalThis
  globalThis['~rari'] = {}

// @ts-expect-error - ~rari is dynamically added to globalThis
globalThis['~rari'].metadataCollector = {
  collect,
}
