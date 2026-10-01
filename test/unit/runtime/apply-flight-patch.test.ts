import type { ReactElement, ReactNode } from 'react'
import { applySoftNavFlightPatch } from '@rari/runtime/flight/apply-flight-patch'
import { flightRouteCache } from '@rari/runtime/flight/route-cache'
import { createElement } from 'react'
import { beforeEach, describe, expect, it } from 'vite-plus/test'

function stamp(path: string, child: ReactNode): ReactElement {
  return createElement('div', { 'data-rari-layout-path': path }, child)
}

function reuse(path: string, child?: ReactNode): ReactElement {
  return createElement('rari-layout-reuse', { 'data-rari-layout-path': path }, child)
}

function htmlDoc(bodyChild: ReactNode): ReactElement {
  return createElement(
    'html',
    { lang: 'en' },
    createElement('head', null, createElement('title', null, 'prev')),
    createElement(
      'body',
      null,
      createElement('nav', null, 'SiteNav'),
      bodyChild,
      createElement('footer', null, 'footer'),
    ),
  )
}

describe('applySoftNavFlightPatch', () => {
  beforeEach(() => {
    flightRouteCache.clear()
  })

  it('segment-ingests without replacing the html shell', () => {
    const previous = htmlDoc(stamp('/', 'home'))
    flightRouteCache.set('/home', '', previous)
    const shellBefore = flightRouteCache.getShell()

    const patched = applySoftNavFlightPatch({
      previousDocument: previous,
      refresh: reuse('/', 'about'),
      fromPathname: '/home',
      toPathname: '/about',
      search: '',
    })

    expect(patched.kind).toBe('merged')
    expect(flightRouteCache.getShell()).toBe(shellBefore)
    expect(flightRouteCache.hasRoute('/about', '')).toBe(true)
    expect(flightRouteCache.hasRoute('/home', '')).toBe(false)
    expect(patched).toEqual({ kind: 'merged', element: previous, maySuspend: false })
  })

  it('signals hard-nav when the layout path stamp is missing', () => {
    const previous = htmlDoc(createElement('main', null, 'home'))
    flightRouteCache.set('/', '', previous)
    const patched = applySoftNavFlightPatch({
      previousDocument: previous,
      refresh: reuse('/', 'about'),
      fromPathname: '/',
      toPathname: '/about',
      search: '',
    })
    expect(patched).toEqual({ kind: 'hard-nav' })
    expect(flightRouteCache.hasRoute('/about', '')).toBe(false)
  })

  it('keeps sibling leaves and parent chrome when soft-navving under a shared parent', () => {
    const pageA = htmlDoc(stamp('/', stamp('/blog', 'a')))
    flightRouteCache.set('/blog/a', '', pageA)
    const chrome = flightRouteCache.readChrome('/')

    const patched = applySoftNavFlightPatch({
      previousDocument: pageA,
      refresh: reuse('/blog', 'b-next'),
      fromPathname: '/blog/a',
      toPathname: '/blog/b',
      search: '',
    })

    expect(patched.kind).toBe('merged')
    expect(flightRouteCache.hasRoute('/blog/a', '')).toBe(false)
    expect(flightRouteCache.hasRoute('/blog/b', '')).toBe(true)
    expect(flightRouteCache.readLeaf('/blog/b', '')).toBe('b-next')
    expect(flightRouteCache.readChrome('/')).toBe(chrome)
  })
})
