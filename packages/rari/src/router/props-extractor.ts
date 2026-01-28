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

export async function extractServerProps(
  componentPath: string,
  params: Record<string, string>,
  searchParams: Record<string, string>,
): Promise<ServerSidePropsResult> {
  try {
    const module = await import(/* @vite-ignore */ componentPath)

    let props: Record<string, any> = {}
    let revalidate: number | undefined
    let notFound = false
    let redirect: string | undefined

    if (typeof module.getData === 'function') {
      const result = await module.getData({ params, searchParams })
      if (result) {
        if (result.notFound)
          notFound = true
        if (result.redirect)
          redirect = result.redirect
        if (result.revalidate !== undefined)
          revalidate = result.revalidate
        if (result.props)
          props = { ...props, ...result.props }
      }
    }

    if (typeof module.getServerSideProps === 'function') {
      const result = await module.getServerSideProps({ params, searchParams })
      if (result) {
        if (result.notFound)
          notFound = true
        if (result.redirect)
          redirect = result.redirect.destination
        if (result.props)
          props = { ...props, ...result.props }
      }
    }

    if (typeof module.getStaticProps === 'function') {
      const result = await module.getStaticProps({ params })
      if (result) {
        if (result.notFound)
          notFound = true
        if (result.redirect)
          redirect = result.redirect.destination
        if (result.revalidate !== undefined)
          revalidate = result.revalidate
        if (result.props)
          props = { ...props, ...result.props }
      }
    }

    return {
      props,
      revalidate,
      notFound,
      redirect,
    }
  }
  catch (error) {
    console.error(`[rari] Router: Failed to extract server props from ${componentPath}:`, error)
    return {
      props: {},
    }
  }
}

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

export function mergeMetadata(
  parentMetadata: MetadataResult,
  childMetadata: MetadataResult,
): MetadataResult {
  const merged: MetadataResult = { ...parentMetadata }

  if (childMetadata.title !== undefined) {
    if (typeof childMetadata.title === 'string') {
      if (typeof parentMetadata.title === 'object' && parentMetadata.title?.template) {
        merged.title = parentMetadata.title.template.replace('%s', childMetadata.title)
      }
      else {
        merged.title = childMetadata.title
      }
    }
    else {
      merged.title = childMetadata.title
    }
  }

  if (childMetadata.description !== undefined)
    merged.description = childMetadata.description
  if (childMetadata.keywords !== undefined)
    merged.keywords = childMetadata.keywords
  if (childMetadata.openGraph !== undefined)
    merged.openGraph = { ...parentMetadata.openGraph, ...childMetadata.openGraph }
  if (childMetadata.twitter !== undefined)
    merged.twitter = { ...parentMetadata.twitter, ...childMetadata.twitter }
  if (childMetadata.robots !== undefined)
    merged.robots = { ...parentMetadata.robots, ...childMetadata.robots }
  if (childMetadata.icons !== undefined)
    merged.icons = { ...parentMetadata.icons, ...childMetadata.icons }
  if (childMetadata.manifest !== undefined)
    merged.manifest = childMetadata.manifest
  if (childMetadata.themeColor !== undefined)
    merged.themeColor = childMetadata.themeColor
  if (childMetadata.appleWebApp !== undefined)
    merged.appleWebApp = { ...parentMetadata.appleWebApp, ...childMetadata.appleWebApp }
  if (childMetadata.viewport !== undefined)
    merged.viewport = childMetadata.viewport
  if (childMetadata.canonical !== undefined)
    merged.canonical = childMetadata.canonical

  return merged
}

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

const propsCache = new Map<string, {
  result: ServerSidePropsResult
  timestamp: number
}>()

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

export function clearPropsCache(): void {
  propsCache.clear()
}

export function clearPropsCacheForComponent(componentPath: string): void {
  for (const key of propsCache.keys()) {
    if (key.startsWith(componentPath))
      propsCache.delete(key)
  }
}

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
