import { isRecord, isStaticParamsArray, warnInvalidStaticParams } from '@/shared/utils/type-guards'

export interface ServerSidePropsResult {
  props: Record<string, any>
  revalidate?: number
  notFound?: boolean
  redirect?: string
}

export interface MetadataResult {
  readonly title?:
    | string
    | {
        readonly default?: string
        readonly template?: string
        readonly absolute?: string
      }
  readonly description?: string
  readonly keywords?: readonly string[]
  readonly openGraph?: {
    readonly title?: string
    readonly description?: string
    readonly url?: string
    readonly siteName?: string
    readonly images?: ReadonlyArray<
      | string
      | {
          readonly url: string
          readonly width?: number
          readonly height?: number
          readonly alt?: string
        }
    >
    readonly type?: string
  }
  readonly twitter?: {
    readonly card?: 'summary' | 'summary_large_image' | 'app' | 'player'
    readonly site?: string
    readonly creator?: string
    readonly title?: string
    readonly description?: string
    readonly images?: readonly string[]
  }
  readonly robots?: {
    readonly index?: boolean
    readonly follow?: boolean
    readonly nocache?: boolean
  }
  readonly icons?: {
    readonly icon?:
      | ReadonlyArray<{
          readonly url: string
          readonly type?: string
          readonly sizes?: string
          readonly rel?: string
        }>
      | string
      | readonly string[]
    readonly apple?:
      | ReadonlyArray<{
          readonly url: string
          readonly sizes?: string
          readonly rel?: string
        }>
      | string
      | readonly string[]
    readonly other?: ReadonlyArray<{
      readonly url: string
      readonly rel?: string
      readonly type?: string
      readonly sizes?: string
      readonly color?: string
    }>
  }
  readonly manifest?: string
  readonly themeColor?:
    | string
    | ReadonlyArray<{
        readonly color: string
        readonly media?: string
      }>
  readonly appleWebApp?: {
    readonly title?: string
    readonly statusBarStyle?: 'default' | 'black' | 'black-translucent'
    readonly capable?: boolean
  }
  readonly viewport?: string
  readonly canonical?: string
  readonly alternates?: {
    readonly canonical?: string
    readonly languages?: { readonly [key: string]: string }
    readonly types?: { readonly [key: string]: string }
  }
}

export type StaticParamsResult = Array<Record<string, string | readonly string[]>>

interface DataFetchResult {
  readonly props?: Record<string, any>
  readonly revalidate?: number
  readonly notFound?: boolean
  readonly redirect?: string | { readonly destination: string }
}

interface ComponentModule {
  readonly getData?: (
    args: Readonly<{
      readonly params: { readonly [key: string]: string }
      readonly searchParams: { readonly [key: string]: string }
    }>,
  ) => DataFetchResult | null | undefined | Promise<DataFetchResult | null | undefined>
  readonly getServerSideProps?: (
    args: Readonly<{
      readonly params: { readonly [key: string]: string }
      readonly searchParams: { readonly [key: string]: string }
    }>,
  ) => DataFetchResult | null | undefined | Promise<DataFetchResult | null | undefined>
  readonly getStaticProps?: (
    args: Readonly<{ readonly params: { readonly [key: string]: string } }>,
  ) => DataFetchResult | null | undefined | Promise<DataFetchResult | null | undefined>
  readonly generateMetadata?: (
    args: Readonly<{
      readonly params: { readonly [key: string]: string }
      readonly searchParams: { readonly [key: string]: string }
    }>,
  ) => unknown
  readonly metadata?: unknown
  readonly generateStaticParams?: () => unknown
}

function isComponentModule(value: unknown): value is ComponentModule {
  return isRecord(value)
}

function isMetadataResult(value: unknown): value is MetadataResult {
  return isRecord(value)
}

async function loadComponentModule(componentPath: string): Promise<ComponentModule> {
  const module: unknown = await import(/* @vite-ignore */ componentPath)
  return isComponentModule(module) ? module : {}
}

/* oxlint-disable typescript/prefer-readonly-parameter-types -- state is an intentional mutable accumulator */
function processGetDataResult(
  result: DataFetchResult | null | undefined,
  state: ServerSidePropsResult,
): void {
  if (!result) return

  if (result.notFound) state.notFound = true
  if (result.redirect != null)
    state.redirect =
      typeof result.redirect === 'string' ? result.redirect : result.redirect.destination
  if (result.revalidate !== undefined) state.revalidate = result.revalidate
  if (result.props) state.props = { ...state.props, ...result.props }
}

async function tryGetData(
  module: ComponentModule,
  params: Readonly<{ readonly [key: string]: string }>,
  searchParams: Readonly<{ readonly [key: string]: string }>,
  state: ServerSidePropsResult,
): Promise<void> {
  if (typeof module.getData !== 'function') return

  const result = await module.getData({ params, searchParams })
  processGetDataResult(result, state)
}

async function tryGetServerSideProps(
  module: ComponentModule,
  params: Readonly<{ readonly [key: string]: string }>,
  searchParams: Readonly<{ readonly [key: string]: string }>,
  state: ServerSidePropsResult,
): Promise<void> {
  if (typeof module.getServerSideProps !== 'function') return

  const result = await module.getServerSideProps({ params, searchParams })
  processGetDataResult(result, state)
}

async function tryGetStaticProps(
  module: ComponentModule,
  params: Readonly<{ readonly [key: string]: string }>,
  state: ServerSidePropsResult,
): Promise<void> {
  if (typeof module.getStaticProps !== 'function') return

  const result = await module.getStaticProps({ params })
  processGetDataResult(result, state)
}
/* oxlint-enable typescript/prefer-readonly-parameter-types */

/* v8 ignore start - requires dynamic imports, better tested in integration/e2e */
export async function extractServerProps(
  componentPath: string,
  params: Readonly<{ readonly [key: string]: string }>,
  searchParams: Readonly<{ readonly [key: string]: string }>,
): Promise<ServerSidePropsResult> {
  try {
    const module = await loadComponentModule(componentPath)

    const state: ServerSidePropsResult = {
      props: {},
    }

    await tryGetData(module, params, searchParams, state)
    await tryGetServerSideProps(module, params, searchParams, state)
    await tryGetStaticProps(module, params, state)

    return state
  } catch (error) {
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
  params: Readonly<{ readonly [key: string]: string }>,
  searchParams: Readonly<{ readonly [key: string]: string }>,
): Promise<MetadataResult> {
  try {
    const module = await loadComponentModule(componentPath)

    if (typeof module.generateMetadata === 'function') {
      const metadata = await module.generateMetadata({ params, searchParams })
      if (isMetadataResult(metadata)) return metadata
    }

    if (isMetadataResult(module.metadata)) return module.metadata

    return {}
  } catch (error) {
    console.error(`[rari] Router: Failed to extract metadata from ${componentPath}:`, error)
    return {}
  }
}
/* v8 ignore stop */

function mergeTitleField(
  parentTitle: MetadataResult['title'],
  childTitle: MetadataResult['title'],
): MetadataResult['title'] {
  if (childTitle === undefined) return parentTitle

  if (typeof childTitle === 'string') {
    if (
      typeof parentTitle === 'object' &&
      parentTitle.template != null &&
      parentTitle.template !== ''
    ) {
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
  if (childField === undefined) return parentField

  return { ...parentField, ...childField }
}

function mergeAlternatesField(
  parent: MetadataResult['alternates'],
  child: MetadataResult['alternates'],
): MetadataResult['alternates'] {
  if (child === undefined) return parent
  if (parent === undefined) return child

  return {
    canonical: child.canonical ?? parent.canonical,
    languages:
      (parent.languages ?? child.languages)
        ? { ...parent.languages, ...child.languages }
        : undefined,
    types: (parent.types ?? child.types) ? { ...parent.types, ...child.types } : undefined,
  }
}

function mergeSimpleField<T>(parentField: T | undefined, childField: T | undefined): T | undefined {
  return childField ?? parentField
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
    alternates: mergeAlternatesField(parentMetadata.alternates, childMetadata.alternates),
  }
}

/* v8 ignore start - requires dynamic imports, better tested in integration/e2e */
export async function extractStaticParams(componentPath: string): Promise<StaticParamsResult> {
  try {
    const module = await loadComponentModule(componentPath)

    if (typeof module.generateStaticParams === 'function') {
      const params = await module.generateStaticParams()
      if (isStaticParamsArray(params)) return params
      warnInvalidStaticParams(componentPath)
    }

    return []
  } catch (error) {
    console.error(`[rari] Router: Failed to extract static params from ${componentPath}:`, error)
    return []
  }
}
/* v8 ignore stop */

/* v8 ignore start - requires dynamic imports, better tested in integration/e2e */
export async function hasServerSideDataFetching(componentPath: string): Promise<boolean> {
  try {
    const module = await loadComponentModule(componentPath)

    return Boolean(
      module.getData ??
      module.getServerSideProps ??
      module.getStaticProps ??
      module.generateMetadata ??
      module.generateStaticParams,
    )
  } catch {
    return false
  }
}
/* v8 ignore stop */

const propsCache = new Map<
  string,
  {
    result: ServerSidePropsResult
    timestamp: number
  }
>()

/* v8 ignore start - depends on extractServerProps which requires dynamic imports */
export async function extractServerPropsWithCache(
  componentPath: string,
  params: Readonly<{ readonly [key: string]: string }>,
  searchParams: Readonly<{ readonly [key: string]: string }>,
  cacheTime: number = 60000,
): Promise<ServerSidePropsResult> {
  const cacheKey = `${componentPath}:${JSON.stringify(params)}:${JSON.stringify(searchParams)}`
  const cached = propsCache.get(cacheKey)

  if (cached && Date.now() - cached.timestamp < cacheTime) return cached.result

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
    if (key.startsWith(componentPath)) propsCache.delete(key)
  }
}
/* v8 ignore stop */

/* v8 ignore start - depends on extractMetadata which requires dynamic imports */
export async function collectMetadataFromChain(
  layoutPaths: readonly string[],
  pagePath: string,
  params: Readonly<{ readonly [key: string]: string }>,
  searchParams: Readonly<{ readonly [key: string]: string }>,
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
