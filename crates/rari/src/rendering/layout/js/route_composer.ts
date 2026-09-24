// oxlint-disable typescript/prefer-readonly-parameter-types
/// <reference path="../../types.d.ts" />

;(function initRouteComposer() {
  if (
    typeof g['~rari']?.composeRoute === 'function' &&
    typeof g['~rari']?.createPageElement === 'function'
  ) {
    return
  }

  type WrapLayoutReuseFn = NonNullable<NonNullable<(typeof g)['~rari']>['wrapLayoutReuse']>
  type RequireCreateElementFn = NonNullable<
    NonNullable<(typeof g)['~rari']>['requireCreateElement']
  >

  function requireWrapLayoutReuse(): WrapLayoutReuseFn {
    const wrap = g['~rari']?.wrapLayoutReuse
    if (typeof wrap !== 'function') {
      throw new TypeError('[rari] wrapLayoutReuse not loaded')
    }
    return wrap
  }

  function requireCreateElement(): ReturnType<RequireCreateElementFn> {
    const require_ = g['~rari']?.requireCreateElement
    if (typeof require_ !== 'function') {
      throw new TypeError('[rari] requireCreateElement not loaded')
    }
    return require_()
  }

  interface LayoutSpec {
    readonly componentId: string
    readonly isRoot: boolean
    readonly filePath: string
    readonly path: string
  }

  interface TemplateSpec {
    readonly componentId: string
    readonly filePath: string
    readonly ssrModuleKey: string
  }

  interface ComposeRouteOptions {
    readonly pageElement: unknown
    readonly layouts?: readonly LayoutSpec[]
    readonly templates?: readonly TemplateSpec[]
    readonly pathname: string
    readonly templateKey: string
    readonly errorComponentId?: string
    readonly metadata?: unknown
    readonly deferRsc?: boolean
    readonly captureStreamId?: string | null
    readonly expandRootLayout?: boolean
    readonly reuseLayoutPaths?: readonly string[]
    readonly actionPostUrl?: unknown
    readonly timings: Record<string, number>
    readonly startTotal: number
  }

  function isThenable(value: unknown): value is PromiseLike<unknown> {
    if (value == null || (typeof value !== 'object' && typeof value !== 'function')) return false
    return typeof Reflect.get(value, 'then') === 'function'
  }

  function resolveTemplateExport(moduleNamespace: unknown): unknown {
    if (moduleNamespace == null || typeof moduleNamespace !== 'object') return null
    const defaultExport: unknown = Reflect.get(moduleNamespace, 'default')
    if (typeof defaultExport === 'function') return defaultExport
    for (const value of Object.values(moduleNamespace)) {
      if (typeof value === 'function') return value
    }
    return null
  }

  function resolveTemplateComponent(componentId: string, ssrModuleKey: string): unknown {
    const direct = g[componentId]
    if (typeof direct === 'function') return direct

    const fromRsc = resolveTemplateExport(g['~rsc']?.modules?.[componentId])
    if (typeof fromRsc === 'function') return fromRsc

    const ssrModules = g['~rari']?.ssrModules
    const fromSsr =
      resolveTemplateExport(ssrModules?.[componentId]) ??
      resolveTemplateExport(ssrModules?.[ssrModuleKey]) ??
      resolveTemplateExport(ssrModules?.[`${ssrModuleKey}#default`])
    if (typeof fromSsr === 'function') return fromSsr

    throw new TypeError(`Template component ${componentId} not found`)
  }

  async function wrapLayout(
    layout: LayoutSpec,
    child: unknown,
    pathname: string,
    expandDocument: boolean,
    reuse: boolean,
    timings: Record<string, number>,
    index: number,
  ): Promise<unknown> {
    const start = performance.now()
    let result: unknown

    if (reuse) {
      result = requireWrapLayoutReuse()(layout.path, child, expandDocument)
    } else {
      const LayoutComponent = g[layout.componentId]
      if (typeof LayoutComponent !== 'function') {
        throw new TypeError(`Layout component ${layout.componentId} not found`)
      }

      const layoutProps = { children: child, pathname }
      if (expandDocument) {
        const invoked: unknown = Reflect.apply(LayoutComponent, undefined, [layoutProps])
        result = isThenable(invoked) ? await invoked : invoked
      } else {
        result = requireCreateElement()(LayoutComponent, layoutProps)
      }
    }

    timings[`layout${index}`] = performance.now() - start
    return result
  }

  function wrapTemplate(
    template: TemplateSpec,
    child: unknown,
    templateKey: string,
    timings: Record<string, number>,
    index: number,
  ): unknown {
    const start = performance.now()
    const TemplateComponent = resolveTemplateComponent(template.componentId, template.ssrModuleKey)
    const result = requireCreateElement()(TemplateComponent, { key: templateKey }, child)
    timings[`template${index}`] = performance.now() - start
    return result
  }

  function wrapErrorBoundary(child: unknown, errorComponentId: string): unknown {
    const ErrorWrapper = {
      $$typeof: Symbol.for('react.client.reference'),
      $$id: 'virtual:error-boundary-wrapper.tsx#ErrorBoundaryWrapper',
      $$async: false,
    }
    return requireCreateElement()(ErrorWrapper, { errorComponentId }, child)
  }

  function isDocumentReuseMarker(value: unknown): boolean {
    if (value == null || typeof value !== 'object') return false
    const type: unknown = Reflect.get(value, 'type')
    if (type !== 'rari-layout-reuse') return false
    const props: unknown = Reflect.get(value, 'props')
    if (props == null || typeof props !== 'object') return false
    return Reflect.get(props, 'data-rari-document-reuse') === true
  }

  async function finalizeComposition(
    element: unknown,
    options: ComposeRouteOptions,
    timings: Record<string, number>,
    startTotal: number,
  ): Promise<unknown> {
    const startRsc = performance.now()
    let elementToRender = element
    const metadata = options.metadata
    const rari = (g['~rari'] ??= {})
    const hasMetadata =
      metadata != null && typeof metadata === 'object' && Object.keys(metadata).length > 0

    if (hasMetadata && isDocumentReuseMarker(elementToRender)) {
      const createElement = requireCreateElement()
      elementToRender = createElement(
        'html',
        null,
        createElement('head', null),
        createElement('body', null, elementToRender),
      )
    }

    if (hasMetadata && typeof rari.injectMetadataIntoDocument === 'function') {
      const injected = rari.injectMetadataIntoDocument(elementToRender, metadata)
      elementToRender = isThenable(injected) ? await injected : injected
    }

    let blockingHeadHtml = ''
    if (typeof rari.hoistBlockingHeadScripts === 'function') {
      const hoisted = rari.hoistBlockingHeadScripts(elementToRender)
      elementToRender = hoisted.element
      blockingHeadHtml = typeof hoisted.html === 'string' ? hoisted.html : ''
    }

    if (options.deferRsc === true) {
      const streamId = options.captureStreamId
      if (streamId != null && streamId !== '') {
        rari.capturedByStream ??= {}
        rari.blockingHeadByStream ??= {}
        rari.capturedByStream[streamId] = elementToRender
        rari.blockingHeadByStream[streamId] = blockingHeadHtml
        return undefined
      }
      if (rari.isActionRefreshCompose === true) {
        rari.actionRefreshElement = elementToRender
      } else {
        rari.capturedElement = elementToRender
      }
      rari.blockingHeadScriptsHtml = blockingHeadHtml
      return undefined
    }

    const renderToRsc = g.renderToRsc
    if (typeof renderToRsc !== 'function') {
      throw new TypeError('[rari] renderToRsc is not available')
    }
    const rscData = await renderToRsc(elementToRender)

    timings.rscConversion = performance.now() - startRsc
    timings.total = performance.now() - startTotal

    const suspense = g['~suspense']
    const boundaries = suspense?.discoveredBoundaries ?? []
    const pendingPromises = suspense?.pendingPromises ?? []

    const result = {
      rsc_data: rscData,
      boundaries,
      pending_promises: pendingPromises,
      has_suspense: boundaries.length > 0 || pendingPromises.length > 0,
      timings,
      metadata: options.metadata ?? {},
      success: true,
    }

    const rsc = (g['~rsc'] ??= {})
    try {
      const cleanResult: unknown = JSON.parse(JSON.stringify(result))
      rsc.renderResult = cleanResult
      return cleanResult
    } catch {
      rsc.renderResult = result
      return result
    }
  }

  async function composeRoute(options: ComposeRouteOptions): Promise<unknown> {
    const rari = (g['~rari'] ??= {})
    if (options.actionPostUrl !== undefined) {
      rari.actionPostUrl =
        typeof options.actionPostUrl === 'string' ? options.actionPostUrl : undefined
    }

    const suspense = (g['~suspense'] ??= {})
    suspense.discoveredBoundaries = []
    suspense.pendingPromises = []
    suspense.promises = {}
    suspense.currentBoundaryId = null

    const timings = options.timings
    const startTotal = options.startTotal
    const reusePaths = new Set(options.reuseLayoutPaths ?? [])
    const layouts = options.layouts ?? []
    const templates = options.templates ?? []
    const pathname = options.pathname
    const templateKey = options.templateKey
    const expandRootLayout = options.expandRootLayout !== false

    let current: unknown = options.pageElement

    for (let i = templates.length - 1, wrapIndex = 0; i >= 0; i -= 1, wrapIndex += 1) {
      const template = templates[i] as TemplateSpec | undefined
      if (template == null) continue
      current = wrapTemplate(template, current, templateKey, timings, wrapIndex)
    }

    const nestedLayouts: LayoutSpec[] = []
    const rootLayouts: LayoutSpec[] = []
    for (const layout of layouts) {
      if (layout.isRoot) rootLayouts.push(layout)
      else nestedLayouts.push(layout)
    }

    let layoutIndex = 0
    for (let i = nestedLayouts.length - 1; i >= 0; i -= 1) {
      const layout = nestedLayouts[i] as LayoutSpec | undefined
      if (layout == null) continue
      current = await wrapLayout(
        layout,
        current,
        pathname,
        false,
        reusePaths.has(layout.path),
        timings,
        layoutIndex,
      )
      layoutIndex += 1
    }

    current = wrapErrorBoundary(current, options.errorComponentId ?? '')

    for (let i = rootLayouts.length - 1; i >= 0; i -= 1) {
      const layout = rootLayouts[i] as LayoutSpec | undefined
      if (layout == null) continue
      current = await wrapLayout(
        layout,
        current,
        pathname,
        expandRootLayout,
        reusePaths.has(layout.path),
        timings,
        layoutIndex,
      )
      layoutIndex += 1
    }

    return finalizeComposition(current, options, timings, startTotal)
  }

  const rari = (g['~rari'] ??= {})
  rari.composeRoute = composeRoute

  type CreatePageElementFn = NonNullable<NonNullable<(typeof g)['~rari']>['createPageElement']>
  type CreatePageElementOptions = Parameters<CreatePageElementFn>[0]

  function createPageElement(options: CreatePageElementOptions): unknown {
    const createElement = requireCreateElement()
    const PageComponent = g[options.pageComponentId]
    if (typeof PageComponent !== 'function') {
      const routeHint =
        options.routeFilePath != null && options.routeFilePath !== ''
          ? ` in route ${options.routeFilePath}`
          : ''
      throw new TypeError(`Page component ${options.pageComponentId} not found${routeHint}`)
    }

    const pageProps = options.pageProps ?? {}
    const loadingId = options.loadingComponentId
    if (loadingId == null || loadingId === '') {
      return createElement(PageComponent, pageProps)
    }

    const LoadingComponent = g[loadingId]
    if (typeof LoadingComponent !== 'function') {
      const loadingHint =
        options.loadingFilePath != null && options.loadingFilePath !== ''
          ? ` in route ${options.loadingFilePath}`
          : ''
      throw new TypeError(`Loading component ${loadingId} not found${loadingHint}`)
    }

    if (options.useSuspense === true) {
      const react = g.React
      const Suspense = react != null ? Reflect.get(react, 'Suspense') : undefined
      if (Suspense == null) {
        throw new TypeError('[rari] React.Suspense is not available')
      }
      return createElement(
        Suspense,
        { fallback: createElement(LoadingComponent, {}) },
        createElement(PageComponent, pageProps),
      )
    }

    return createElement(PageComponent, pageProps)
  }

  rari.createPageElement = createPageElement
})()
