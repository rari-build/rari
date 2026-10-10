import type { ReactElement, ReactNode } from 'react'
import { applySoftNavFlightPatch } from '@rari/runtime/flight/apply-flight-patch'
import { renderFlightLayoutRouter } from '@rari/runtime/flight/layout-router'
import { containsLayoutSlot, flightRouteCache } from '@rari/runtime/flight/route-cache'
import { createElement, isValidElement, Suspense } from 'react'
import { beforeEach, describe, expect, it } from 'vite-plus/test'

function stamp(path: string, child: ReactNode): ReactElement {
  return createElement(
    'div',
    { 'data-rari-layout-path': path, 'style': { display: 'contents' } },
    child,
  )
}

function reuse(path: string, child?: ReactNode): ReactElement {
  return createElement('rari-layout-reuse', { 'data-rari-layout-path': path }, child)
}

function playgroundDoc(page: ReactNode): ReactElement {
  return createElement(
    'html',
    { lang: 'en' },
    createElement('head', null, createElement('title', null, 'doc')),
    createElement(
      'body',
      null,
      createElement(
        'div',
        { className: 'shell' },
        createElement('nav', null, 'SiteNav'),
        createElement('main', null, stamp('/', page)),
      ),
    ),
  )
}

function page(text: string): ReactElement {
  return createElement('section', { 'data-page': text }, text)
}

function suspenseFallbackChildren(node: ReactElement): unknown {
  const props: unknown = node.props
  if (typeof props !== 'object' || props == null || !('fallback' in props)) return undefined
  const fallback: unknown = Reflect.get(props, 'fallback')
  if (!isValidElement(fallback)) return undefined
  const fallbackProps: unknown = fallback.props
  if (
    typeof fallbackProps !== 'object' ||
    fallbackProps == null ||
    !('children' in fallbackProps)
  ) {
    return undefined
  }
  return Reflect.get(fallbackProps, 'children')
}

describe('soft-nav hollow-shell paint', () => {
  beforeEach(() => {
    flightRouteCache.clear()
  })

  it('soft-nav keeps a real document for payload; cache holds shell + new leaf', () => {
    const home = playgroundDoc(page('home'))
    flightRouteCache.set('/', '', home)

    const patched = applySoftNavFlightPatch({
      previousDocument: home,
      refresh: reuse('/', page('about')),
      fromPathname: '/',
      toPathname: '/about',
      fromSearch: '',
      search: '',
    })

    expect(patched.kind).toBe('merged')
    if (patched.kind !== 'merged') return
    expect(containsLayoutSlot(patched.element)).toBe(false)
    expect(patched.element).toBe(home)
    expect(containsLayoutSlot(flightRouteCache.getShell())).toBe(true)
    expect(flightRouteCache.hasRoute('/about', '')).toBe(true)
    expect(flightRouteCache.readLeaf('/about', '')).toEqual(page('about'))
  })

  it('after soft-nav, FlightLayoutRouter at / paints the new leaf in a stable Suspense', () => {
    const home = playgroundDoc(page('home'))
    flightRouteCache.set('/', '', home)

    applySoftNavFlightPatch({
      previousDocument: home,
      refresh: reuse('/', page('about')),
      fromPathname: '/',
      toPathname: '/about',
      fromSearch: '',
      search: '',
    })

    const rendered = renderFlightLayoutRouter({
      layoutPath: '/',
      pathname: '/about',
      search: '',
    })
    expect(isValidElement(rendered)).toBe(true)
    if (!isValidElement(rendered)) return
    expect(rendered.type).toBe(Suspense)
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    expect((rendered.props as { children?: unknown }).children).toEqual(page('about'))
  })

  it('keeps a stable Suspense when soft-nav swaps a bare page for a loading boundary', () => {
    const home = playgroundDoc(page('home'))
    flightRouteCache.set('/', '', home)

    const loadingLeaf = createElement(
      Suspense,
      { fallback: createElement('div', null, 'loading') },
      page('server-data'),
    )
    applySoftNavFlightPatch({
      previousDocument: home,
      refresh: reuse('/', loadingLeaf),
      fromPathname: '/',
      toPathname: '/server-data',
      fromSearch: '',
      search: '',
    })

    const fromHome = renderFlightLayoutRouter({
      layoutPath: '/',
      pathname: '/',
      search: '',
    })
    const toServerData = renderFlightLayoutRouter({
      layoutPath: '/',
      pathname: '/server-data',
      search: '',
    })
    expect(isValidElement(fromHome) && fromHome.type === Suspense).toBe(true)
    expect(isValidElement(toServerData) && toServerData.type === Suspense).toBe(true)
    if (!isValidElement(fromHome) || !isValidElement(toServerData)) return
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    expect((fromHome.props as { children?: unknown }).children).toEqual(page('home'))
    expect(suspenseFallbackChildren(toServerData)).toBe('loading')
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    expect((toServerData.props as { children?: unknown }).children).toEqual(page('server-data'))
  })

  it('detects loading.tsx through layout-reuse + ErrorBoundary (soft-nav flight shape)', async () => {
    const { leafHasLoadingFallback } = await import('@rari/runtime/flight/react-helpers')
    function ErrorBoundaryWrapper(props: {
      readonly errorComponentId: string
      readonly children?: ReactNode
    }): ReactElement {
      return createElement('div', { 'data-error-boundary': props.errorComponentId }, props.children)
    }
    const refresh = reuse(
      '/',
      createElement(
        ErrorBoundaryWrapper,
        { errorComponentId: '' },
        createElement(
          Suspense,
          { fallback: createElement('div', null, 'loading') },
          page('server-data'),
        ),
      ),
    )
    expect(leafHasLoadingFallback(refresh)).toBe(true)
  })

  it('detects loading.tsx through template-style composite wrappers', async () => {
    const { leafHasLoadingFallback, unwrapLoadingSuspense } =
      await import('@rari/runtime/flight/react-helpers')
    function Template({ children }: { readonly children?: ReactNode }): ReactElement {
      return createElement('div', { 'data-template': true }, children)
    }
    const leaf = createElement(
      Template,
      null,
      createElement(Suspense, { fallback: createElement('div', null, 'loading') }, page('about')),
    )
    expect(leafHasLoadingFallback(leaf)).toBe(true)
    const unwrapped = unwrapLoadingSuspense(leaf)
    expect(unwrapped.fallback).not.toBeNull()
    expect(isValidElement(unwrapped.content) && unwrapped.content.type === Template).toBe(true)
  })

  it('does not hoist loading Suspense above host DOM chrome', async () => {
    const { leafHasLoadingFallback } = await import('@rari/runtime/flight/react-helpers')
    const leaf = createElement(
      'div',
      { className: 'chrome' },
      createElement(Suspense, { fallback: createElement('div', null, 'loading') }, page('about')),
    )
    expect(leafHasLoadingFallback(leaf)).toBe(false)
  })

  it('soft-nav leaf is not clobbered when previousDocument would be re-set under the new path', () => {
    const home = playgroundDoc(page('home'))
    flightRouteCache.set('/', '', home)
    applySoftNavFlightPatch({
      previousDocument: home,
      refresh: reuse('/', page('server-data')),
      fromPathname: '/',
      toPathname: '/server-data',
      fromSearch: '',
      search: '',
    })
    expect(flightRouteCache.readLeaf('/server-data', '')).toEqual(page('server-data'))

    if (flightRouteCache.getShell() == null || !flightRouteCache.hasRoute('/server-data', '')) {
      flightRouteCache.set('/server-data', '', home)
    }
    expect(flightRouteCache.readLeaf('/server-data', '')).toEqual(page('server-data'))
  })

  it('hoists loading Suspense through ErrorBoundaryWrapper (composeRoute leaf shape)', () => {
    const home = playgroundDoc(page('home'))
    flightRouteCache.set('/', '', home)

    function ErrorBoundaryWrapper(props: {
      readonly errorComponentId: string
      readonly children?: ReactNode
    }): ReactElement {
      return createElement('div', { 'data-error-boundary': props.errorComponentId }, props.children)
    }

    const loadingLeaf = createElement(
      ErrorBoundaryWrapper,
      { errorComponentId: '' },
      createElement(
        Suspense,
        { fallback: createElement('div', null, 'loading') },
        page('server-data'),
      ),
    )
    applySoftNavFlightPatch({
      previousDocument: home,
      refresh: reuse('/', loadingLeaf),
      fromPathname: '/',
      toPathname: '/server-data',
      fromSearch: '',
      search: '',
    })

    const toServerData = renderFlightLayoutRouter({
      layoutPath: '/',
      pathname: '/server-data',
      search: '',
    })
    expect(isValidElement(toServerData) && toServerData.type === Suspense).toBe(true)
    if (!isValidElement(toServerData)) return
    expect(suspenseFallbackChildren(toServerData)).toBe('loading')
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    const content = (toServerData.props as { children?: ReactElement }).children
    expect(isValidElement(content) && content.type === ErrorBoundaryWrapper).toBe(true)
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    expect((content as ReactElement<{ children?: unknown }>).props.children).toEqual(
      page('server-data'),
    )
  })

  it('marks maySuspend only when a thenable remains, not a resolved Suspense', async () => {
    const { Suspense } = await import('react')
    const { flightTreeMaySuspend } = await import('@rari/runtime/flight/react-helpers')
    const resolved = reuse('/', createElement(Suspense, null, page('blog')))
    expect(flightTreeMaySuspend(resolved)).toBe(false)

    const fulfilled = Object.assign(Promise.resolve(page('done')), { status: 'fulfilled' })
    expect(flightTreeMaySuspend(reuse('/', createElement(Suspense, null, fulfilled)))).toBe(false)

    const pending = reuse(
      '/',
      createElement(
        Suspense,
        null,
        Object.assign(Promise.resolve(page('slow')), { status: 'pending' }),
      ),
    )
    expect(flightTreeMaySuspend(pending)).toBe(true)
  })

  it('ignores client-reference lazy wrappers when deciding maySuspend', async () => {
    const { flightTreeMaySuspend } = await import('@rari/runtime/flight/react-helpers')
    const lazyType = {
      $$typeof: Symbol.for('react.lazy'),
      _payload: Object.assign(Promise.resolve('PageTransition'), { status: 'pending' }),
      _init: () => 'PageTransition',
    }
    const withClientLazy = reuse(
      '/',
      // oxlint-disable-next-line typescript/no-unsafe-type-assertion
      createElement(lazyType as never, null, page('about')),
    )
    expect(flightTreeMaySuspend(withClientLazy)).toBe(false)
  })

  it('marks maySuspend for pending lazy holes inside loading Suspense', async () => {
    const { Suspense } = await import('react')
    const { flightTreeMaySuspend } = await import('@rari/runtime/flight/react-helpers')
    const lazyType = {
      $$typeof: Symbol.for('react.lazy'),
      _payload: Object.assign(Promise.resolve(page('blog')), { status: 'pending' }),
      _init: () => page('blog'),
    }
    const withLoadingSuspense = reuse(
      '/',
      createElement(
        Suspense,
        { fallback: createElement('div', null, 'loading') },
        // oxlint-disable-next-line typescript/no-unsafe-type-assertion
        createElement(lazyType as never),
      ),
    )
    expect(flightTreeMaySuspend(withLoadingSuspense)).toBe(true)
  })

  it('marks maySuspend when Suspense children are a bare pending react.lazy type', async () => {
    const { Suspense } = await import('react')
    const { flightTreeMaySuspend, leafHasLoadingFallback } =
      await import('@rari/runtime/flight/react-helpers')
    const bareLazyHole = {
      $$typeof: Symbol.for('react.lazy'),
      _payload: Object.assign(Promise.resolve(page('server-data')), { status: 'pending' }),
      _init: () => page('server-data'),
    }
    const leaf = createElement(
      Suspense,
      { fallback: createElement('div', null, 'loading') },
      // oxlint-disable-next-line typescript/no-unsafe-type-assertion
      bareLazyHole as never,
    )
    expect(flightTreeMaySuspend(leaf)).toBe(true)
    expect(leafHasLoadingFallback(leaf)).toBe(true)
  })

  it('marks maySuspend for Flight resolved_model / resolved_module (can still block on init)', async () => {
    const { Suspense } = await import('react')
    const { flightTreeMaySuspend } = await import('@rari/runtime/flight/react-helpers')
    for (const status of ['resolved_model', 'resolved_module'] as const) {
      const bareLazyHole = {
        $$typeof: Symbol.for('react.lazy'),
        _payload: Object.assign(Promise.resolve(page('interactive')), { status }),
        _init: () => page('interactive'),
      }
      const leaf = createElement(
        Suspense,
        { fallback: createElement('div', null, 'loading') },
        // oxlint-disable-next-line typescript/no-unsafe-type-assertion
        bareLazyHole as never,
      )
      expect(flightTreeMaySuspend(leaf)).toBe(true)
    }
  })

  it('does not mark maySuspend for fulfilled Flight payloads', async () => {
    const { Suspense } = await import('react')
    const { flightTreeMaySuspend } = await import('@rari/runtime/flight/react-helpers')
    const bareLazyHole = {
      $$typeof: Symbol.for('react.lazy'),
      _payload: Object.assign(Promise.resolve(page('interactive')), { status: 'fulfilled' }),
      _init: () => page('interactive'),
    }
    const leaf = createElement(
      Suspense,
      { fallback: createElement('div', null, 'loading') },
      // oxlint-disable-next-line typescript/no-unsafe-type-assertion
      bareLazyHole as never,
    )
    expect(flightTreeMaySuspend(leaf)).toBe(false)
  })

  it('does not mark maySuspend for pending client wrappers with ready children', async () => {
    const { Suspense } = await import('react')
    const { flightTreeMaySuspend } = await import('@rari/runtime/flight/react-helpers')
    const clientLazy = {
      $$typeof: Symbol.for('react.lazy'),
      _payload: Object.assign(Promise.resolve('PageTransition'), { status: 'pending' }),
      _init: () => 'PageTransition',
    }
    expect(
      flightTreeMaySuspend(
        reuse(
          '/',
          createElement(
            Suspense,
            { fallback: createElement('div', null, 'loading') },
            // oxlint-disable-next-line typescript/no-unsafe-type-assertion
            createElement(clientLazy as never, null, page('blog')),
          ),
        ),
      ),
    ).toBe(false)
  })

  it('does not mark maySuspend for pending client leaves beside ready RSC siblings', async () => {
    const { Suspense } = await import('react')
    const { flightTreeMaySuspend } = await import('@rari/runtime/flight/react-helpers')
    const pageTransition = {
      $$typeof: Symbol.for('react.lazy'),
      _payload: Object.assign(Promise.resolve('PageTransition'), { status: 'pending' }),
      _init: () => 'PageTransition',
    }
    const counter = {
      $$typeof: Symbol.for('react.lazy'),
      _payload: Object.assign(Promise.resolve('Counter'), { status: 'pending' }),
      _init: () => 'Counter',
    }
    expect(
      flightTreeMaySuspend(
        reuse(
          '/',
          createElement(
            Suspense,
            { fallback: createElement('div', null, 'loading') },
            createElement(
              // oxlint-disable-next-line typescript/no-unsafe-type-assertion
              pageTransition as never,
              null,
              page('interactive'),
              // oxlint-disable-next-line typescript/no-unsafe-type-assertion
              createElement(counter as never),
            ),
          ),
        ),
      ),
    ).toBe(false)
  })

  it('does not mark maySuspend for fulfilled lazy holes', async () => {
    const { Suspense } = await import('react')
    const { flightTreeMaySuspend } = await import('@rari/runtime/flight/react-helpers')
    const fulfilledLazy = {
      $$typeof: Symbol.for('react.lazy'),
      _payload: Object.assign(Promise.resolve(page('blog')), {
        status: 'fulfilled',
        value: page('blog'),
      }),
      _init: () => page('blog'),
    }
    expect(
      flightTreeMaySuspend(
        reuse(
          '/',
          createElement(
            Suspense,
            { fallback: createElement('div', null, 'loading') },
            // oxlint-disable-next-line typescript/no-unsafe-type-assertion
            createElement(fulfilledLazy as never),
          ),
        ),
      ),
    ).toBe(false)
  })

  it('marks maySuspend for unstamped thenables inside loading Suspense', async () => {
    const { Suspense } = await import('react')
    const { flightTreeMaySuspend } = await import('@rari/runtime/flight/react-helpers')
    const unstamped = {
      $$typeof: Symbol.for('react.lazy'),
      _payload: Promise.resolve(page('blog')),
      _init: () => page('blog'),
    }
    expect(
      flightTreeMaySuspend(
        reuse(
          '/',
          createElement(
            Suspense,
            { fallback: createElement('div', null, 'loading') },
            // oxlint-disable-next-line typescript/no-unsafe-type-assertion
            createElement(unstamped as never),
          ),
        ),
      ),
    ).toBe(true)

    expect(
      flightTreeMaySuspend(
        reuse(
          '/',
          createElement(
            Suspense,
            { fallback: createElement('div', null, 'loading') },
            Promise.resolve(page('blog')),
          ),
        ),
      ),
    ).toBe(true)
  })
})
