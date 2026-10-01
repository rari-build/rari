import type { ReactElement, ReactNode } from 'react'
import { applySoftNavFlightPatch } from '@rari/runtime/flight/apply-flight-patch'
import { FlightLayoutRouter } from '@rari/runtime/flight/layout-router'
import { containsLayoutSlot, flightRouteCache } from '@rari/runtime/flight/route-cache'
import { createElement, isValidElement } from 'react'
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

  it('after soft-nav, FlightLayoutRouter at / paints the new leaf', () => {
    const home = playgroundDoc(page('home'))
    flightRouteCache.set('/', '', home)

    applySoftNavFlightPatch({
      previousDocument: home,
      refresh: reuse('/', page('about')),
      fromPathname: '/',
      toPathname: '/about',
      search: '',
    })

    const rendered = FlightLayoutRouter({
      layoutPath: '/',
      pathname: '/about',
      search: '',
      revision: 1,
    })
    expect(isValidElement(rendered)).toBe(true)
    if (!isValidElement(rendered)) return
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    const props = rendered.props as { children?: ReactNode }
    expect(isValidElement(props.children)).toBe(true)
    if (!isValidElement(props.children)) return
    expect(props.children.key).toBe('/about')
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    const keyed = props.children.props as { children?: ReactNode }
    expect(keyed.children).toEqual(page('about'))
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
})
