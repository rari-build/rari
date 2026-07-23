/// <reference path="../core/types.d.ts" />

interface MetadataParams {
  readonly params: Readonly<Record<string, string>>
  readonly searchParams: Readonly<Record<string, string>>
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
  layoutPaths: readonly string[],
  pagePath: string,
  params: Readonly<Record<string, string>>,
  searchParams: Readonly<Record<string, string>>,
): Promise<PageMetadata[]> {
  const metadataList: PageMetadata[] = []

  async function extractMetadata(
    modulePath: string,
    params: Readonly<Record<string, string>>,
    searchParams: Readonly<Record<string, string>>,
  ): Promise<PageMetadata> {
    try {
      if (g['~rsc']?.modules !== undefined) {
        const moduleKey = modulePath.replace(FILE_URL_REGEX, 'app/').replace(JS_EXTENSION_REGEX, '')

        // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- RSC module table is dynamically populated
        const module = g['~rsc'].modules[moduleKey] as ModuleWithMetadata | undefined

        if (module) {
          if (typeof module.generateMetadata === 'function') {
            const result = await module.generateMetadata({ params, searchParams })
            return result
          }

          if (module.metadata != null && typeof module.metadata === 'object') return module.metadata
        }
      }

      // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- route modules are loaded dynamically at runtime
      const module = (await import(modulePath)) as ModuleWithMetadata

      if (typeof module.generateMetadata === 'function') {
        const result = await module.generateMetadata({ params, searchParams })
        return result
      }

      if (module.metadata != null && typeof module.metadata === 'object') return module.metadata

      return {}
    } catch (error) {
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
}

g['~rari'] ??= {}

g['~rari'].metadataCollector = {
  collect,
}
