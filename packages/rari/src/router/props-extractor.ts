export interface ServerSidePropsResult {
  props: Record<string, any>
  revalidate?: number
  notFound?: boolean
  redirect?: string
}

export interface MetadataResult {
  title?: string | {
    default?: string
    template?: string
    absolute?: string
  }
  description?: string
  keywords?: string[]
  openGraph?: {
    title?: string
    description?: string
    url?: string
    siteName?: string
    images?: Array<string | {
      url: string
      width?: number
      height?: number
      alt?: string
    }>
    type?: string
  }
  twitter?: {
    card?: 'summary' | 'summary_large_image' | 'app' | 'player'
    site?: string
    creator?: string
    title?: string
    description?: string
    images?: string[]
  }
  robots?: {
    index?: boolean
    follow?: boolean
    nocache?: boolean
  }
  icons?: {
    icon?: Array<{
      url: string
      type?: string
      sizes?: string
      rel?: string
    }> | string | string[]
    apple?: Array<{
      url: string
      sizes?: string
      rel?: string
    }> | string | string[]
    other?: Array<{
      url: string
      rel?: string
      type?: string
      sizes?: string
      color?: string
    }>
  }
  manifest?: string
  themeColor?: string | Array<{
    color: string
    media?: string
  }>
  appleWebApp?: {
    title?: string
    statusBarStyle?: 'default' | 'black' | 'black-translucent'
    capable?: boolean
  }
  viewport?: string
  canonical?: string
}

export type StaticParamsResult = Array<Record<string, string | string[]>>

interface DataFetchResult {
  props?: Record<string, any>
  revalidate?: number
  notFound?: boolean
  redirect?: string | { destination: string }
}

function processGetDataResult(result: DataFetchResult | null | undefined, state: ServerSidePropsResult): void {
  if (!result)
    return

  if (result.notFound)
    state.notFound = true
  if (result.redirect)
    state.redirect = typeof result.redirect === 'string' ? result.redirect : result.redirect.destination
  if (result.revalidate !== undefined)
    state.revalidate = result.revalidate
  if (result.props)
    state.props = { ...state.props, ...result.props }
}

async function tryGetData(module: any, params: Record<string, string>, searchParams: Record<string, string>, state: ServerSidePropsResult): Promise<void> {
  if (typeof module.getData !== 'function')
    return

  const result = await module.getData({ params, searchParams })
  processGetDataResult(result, state)
}

async function tryGetServerSideProps(module: any, params: Record<string, string>, searchParams: Record<string, string>, state: ServerSidePropsResult): Promise<void> {
  if (typeof module.getServerSideProps !== 'function')
    return

  const result = await module.getServerSideProps({ params, searchParams })
  processGetDataResult(result, state)
}

async function tryGetStaticProps(module: any, params: Record<string, string>, state: ServerSidePropsResult): Promise<void> {
  if (typeof module.getStaticProps !== 'function')
    return

  const result = await module.getStaticProps({ params })
  processGetDataResult(result, state)
}

/* v8 ignore start - requires dynamic imports, better tested in integration/e2e */
export async function extractServerProps(
  componentPath: string,
  params: Record<string, string>,
  searchParams: Record<string, string>,
): Promise<ServerSidePropsResult> {
  try {
    const module = await import(/* @vite-ignore */ componentPath)

    const state: ServerSidePropsResult = {
      props: {},
    }

    await tryGetData(module, params, searchParams, state)
    await tryGetServerSideProps(module, params, searchParams, state)
    await tryGetStaticProps(module, params, state)

    return state
  }
  catch (error) {
    console.error(`[rari] Router: Failed to extract server props from ${componentPath}:`, error)
    return {
      props: {},
    }
  }
}
/* v8 ignore stop */

/* v8 ignore start - requires dynamic imports, better tested in integration/e2e */
export async function extractMetadata(
  componentPath: string,
  params: Record<string, string>,
  searchParams: Record<string, string>,
): Promise<MetadataResult> {
  try {
    const module = await import(/* @vite-ignore */ componentPath)

    if (typeof module.generateMetadata === 'function') {
      const metadata = await module.generateMetadata({ params, searchParams })
      if (metadata && typeof metadata === 'object')
        return metadata
    }

    if (module.metadata && typeof module.metadata === 'object')
      return module.metadata

    return {}
  }
  catch (error) {
    console.error(`[rari] Router: Failed to extract metadata from ${componentPath}:`, error)
    return {}
  }
}
/* v8 ignore stop */

function mergeTitleField(parentTitle: MetadataResult['title'], childTitle: MetadataResult['title']): MetadataResult['title'] {
  if (childTitle === undefined)
    return parentTitle

  if (typeof childTitle === 'string') {
    if (typeof parentTitle === 'object' && parentTitle?.template) {
      const hasPlaceholder = parentTitle.template.includes('%s')
      return hasPlaceholder ? parentTitle.template.replace('%s', childTitle) : childTitle
    }

    return childTitle
  }

  return childTitle
}

function mergeObjectField<T extends Record<string, any>>(
  parentField: T | undefined,
  childField: T | undefined,
): T | undefined {
  if (childField === undefined)
    return parentField

  return { ...parentField, ...childField } as T
}

function mergeSimpleField<T>(parentField: T | undefined, childField: T | undefined): T | undefined {
  return childField !== undefined ? childField : parentField
}

export function mergeMetadata(
  parentMetadata: MetadataResult,
  childMetadata: MetadataResult,
): MetadataResult {
  return {
    ...parentMetadata,
    title: mergeTitleField(parentMetadata.title, childMetadata.title),
    description: mergeSimpleField(parentMetadata.description, childMetadata.description),
    keywords: mergeSimpleField(parentMetadata.keywords, childMetadata.keywords),
    openGraph: mergeObjectField(parentMetadata.openGraph, childMetadata.openGraph),
    twitter: mergeObjectField(parentMetadata.twitter, childMetadata.twitter),
    robots: mergeObjectField(parentMetadata.robots, childMetadata.robots),
    icons: mergeObjectField(parentMetadata.icons, childMetadata.icons),
    manifest: mergeSimpleField(parentMetadata.manifest, childMetadata.manifest),
    themeColor: mergeSimpleField(parentMetadata.themeColor, childMetadata.themeColor),
    appleWebApp: mergeObjectField(parentMetadata.appleWebApp, childMetadata.appleWebApp),
    viewport: mergeSimpleField(parentMetadata.viewport, childMetadata.viewport),
    canonical: mergeSimpleField(parentMetadata.canonical, childMetadata.canonical),
  }
}

/* v8 ignore start - requires dynamic imports, better tested in integration/e2e */
export async function extractStaticParams(
  componentPath: string,
): Promise<StaticParamsResult> {
  try {
    const module = await import(/* @vite-ignore */ componentPath)

    if (typeof module.generateStaticParams === 'function') {
      const params = await module.generateStaticParams()
      if (Array.isArray(params))
        return params
    }

    return []
  }
  catch (error) {
    console.error(`[rari] Router: Failed to extract static params from ${componentPath}:`, error)
    return []
  }
}
/* v8 ignore stop */

/* v8 ignore start - requires dynamic imports, better tested in integration/e2e */
export async function hasServerSideDataFetching(
  componentPath: string,
): Promise<boolean> {
  try {
    const module = await import(/* @vite-ignore */ componentPath)

    return !!(
      module.getData
      || module.getServerSideProps
      || module.getStaticProps
      || module.generateMetadata
      || module.generateStaticParams
    )
  }
  catch {
    return false
  }
}
/* v8 ignore stop */

const propsCache = new Map<string, {
  result: ServerSidePropsResult
  timestamp: number
}>()

/* v8 ignore start - depends on extractServerProps which requires dynamic imports */
export async function extractServerPropsWithCache(
  componentPath: string,
  params: Record<string, string>,
  searchParams: Record<string, string>,
  cacheTime: number = 60000,
): Promise<ServerSidePropsResult> {
  const cacheKey = `${componentPath}:${JSON.stringify(params)}:${JSON.stringify(searchParams)}`
  const cached = propsCache.get(cacheKey)

  if (cached && Date.now() - cached.timestamp < cacheTime)
    return cached.result

  const result = await extractServerProps(componentPath, params, searchParams)

  propsCache.set(cacheKey, {
    result,
    timestamp: Date.now(),
  })

  return result
}
/* v8 ignore stop */

export function clearPropsCache(): void {
  propsCache.clear()
}

/* v8 ignore start - cache clearing logic, difficult to test without populating cache via dynamic imports */
export function clearPropsCacheForComponent(componentPath: string): void {
  for (const key of propsCache.keys()) {
    if (key.startsWith(componentPath))
      propsCache.delete(key)
  }
}
/* v8 ignore stop */

/* v8 ignore start - depends on extractMetadata which requires dynamic imports */
export async function collectMetadataFromChain(
  layoutPaths: string[],
  pagePath: string,
  params: Record<string, string>,
  searchParams: Record<string, string>,
): Promise<MetadataResult> {
  let metadata: MetadataResult = {}

  for (const layoutPath of layoutPaths) {
    const layoutMetadata = await extractMetadata(layoutPath, params, searchParams)
    metadata = mergeMetadata(metadata, layoutMetadata)
  }

  const pageMetadata = await extractMetadata(pagePath, params, searchParams)
  metadata = mergeMetadata(metadata, pageMetadata)

  return metadata
}
/* v8 ignore stop */
