export interface ServerPropsResult {
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
    images?: Array<{
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
    googleBot?: {
      'index'?: boolean
      'follow'?: boolean
      'noimageindex'?: boolean
      'max-video-preview'?: number
      'max-image-preview'?: 'none' | 'standard' | 'large'
      'max-snippet'?: number
    }
  }
  icons?: {
    icon?: string | string[]
    shortcut?: string | string[]
    apple?: string | string[]
  }
  manifest?: string
  viewport?: {
    width?: string | number
    height?: string | number
    initialScale?: number
    maximumScale?: number
    minimumScale?: number
    userScalable?: boolean
  }
  verification?: {
    google?: string
    yandex?: string
    yahoo?: string
    other?: Record<string, string>
  }
  alternates?: {
    canonical?: string
    languages?: Record<string, string>
  }
}

export type StaticParamsResult = Array<Record<string, string | string[]>>

export async function extractServerProps(
  componentPath: string,
  params: Record<string, string>,
  searchParams: Record<string, string>,
): Promise<ServerPropsResult> {
  try {
    const module = await import(/* @vite-ignore */ componentPath)

    let props: Record<string, any> = {}
    let revalidate: number | undefined
    let notFound = false
    let redirect: string | undefined

    if (typeof module.getData === 'function') {
      const result = await module.getData({ params, searchParams })
      if (result) {
        if (result.notFound) {
          notFound = true
        }
        if (result.redirect) {
          redirect = result.redirect
        }
        if (result.revalidate !== undefined) {
          revalidate = result.revalidate
        }
        if (result.props) {
          props = { ...props, ...result.props }
        }
      }
    }

    if (typeof module.getServerSideProps === 'function') {
      const result = await module.getServerSideProps({ params, searchParams })
      if (result) {
        if (result.notFound) {
          notFound = true
        }
        if (result.redirect) {
          redirect = result.redirect.destination
        }
        if (result.props) {
          props = { ...props, ...result.props }
        }
      }
    }

    if (typeof module.getStaticProps === 'function') {
      const result = await module.getStaticProps({ params })
      if (result) {
        if (result.notFound) {
          notFound = true
        }
        if (result.redirect) {
          redirect = result.redirect.destination
        }
        if (result.revalidate !== undefined) {
          revalidate = result.revalidate
        }
        if (result.props) {
          props = { ...props, ...result.props }
        }
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
    console.error(`Failed to extract server props from ${componentPath}:`, error)
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

    if (module.metadata && typeof module.metadata === 'object') {
      return module.metadata
    }

    if (typeof module.generateMetadata === 'function') {
      const metadata = await module.generateMetadata({ params, searchParams })
      if (metadata && typeof metadata === 'object') {
        return metadata
      }
    }

    return {}
  }
  catch (error) {
    console.error(`Failed to extract metadata from ${componentPath}:`, error)
    return {}
  }
}

export async function extractStaticParams(
  componentPath: string,
): Promise<StaticParamsResult> {
  try {
    const module = await import(/* @vite-ignore */ componentPath)

    if (typeof module.generateStaticParams === 'function') {
      const params = await module.generateStaticParams()
      if (Array.isArray(params)) {
        return params
      }
    }

    return []
  }
  catch (error) {
    console.error(`Failed to extract static params from ${componentPath}:`, error)
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
  result: ServerPropsResult
  timestamp: number
}>()

export async function extractServerPropsWithCache(
  componentPath: string,
  params: Record<string, string>,
  searchParams: Record<string, string>,
  cacheTime: number = 60000, // 1 minute default
): Promise<ServerPropsResult> {
  const cacheKey = `${componentPath}:${JSON.stringify(params)}:${JSON.stringify(searchParams)}`
  const cached = propsCache.get(cacheKey)

  if (cached && Date.now() - cached.timestamp < cacheTime) {
    return cached.result
  }

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
    if (key.startsWith(componentPath)) {
      propsCache.delete(key)
    }
  }
}
