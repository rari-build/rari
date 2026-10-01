import type { ReactElement, ReactNode } from 'react'
import { FlightDocument, FlightLayoutRouter } from '@rari/runtime/flight/layout-router'
import { flightRouteCache } from '@rari/runtime/flight/route-cache'
import { createElement } from 'react'
import { beforeEach, describe, expect, it } from 'vite-plus/test'

function stamp(path: string, child: ReactNode): ReactElement {
  return createElement('div', { 'data-rari-layout-path': path }, child)
}

function htmlDoc(bodyChild: ReactNode): ReactElement {
  return createElement(
    'html',
    { lang: 'en' },
    createElement('head', null),
    createElement('body', null, bodyChild),
  )
}

describe('flightLayoutRouter / FlightDocument', () => {
  beforeEach(() => {
    flightRouteCache.clear()
  })

  it('parent chrome stays stable while nested leaf is deepest', () => {
    const page = createElement('main', null, 'page-a')
    flightRouteCache.set('/blog/a', '', htmlDoc(stamp('/', stamp('/blog', page))))

    expect(flightRouteCache.isDeepestLayout('/', '/blog/a')).toBe(false)
    expect(flightRouteCache.isDeepestLayout('/blog', '/blog/a')).toBe(true)
    expect(flightRouteCache.readChrome('/')).not.toBeNull()
    expect(flightRouteCache.readLeaf('/blog/a', '')).toEqual(page)
  })

  it('ingestSegment under a nested layout keeps parent chrome', () => {
    const pageA = createElement('main', null, 'a')
    const pageB = createElement('main', null, 'b')
    flightRouteCache.set('/blog/a', '', htmlDoc(stamp('/', stamp('/blog', pageA))))
    const chrome = flightRouteCache.readChrome('/')

    expect(
      flightRouteCache.ingestSegment(
        createElement('rari-layout-reuse', { 'data-rari-layout-path': '/blog' }, pageB),
        '/blog/b',
        '',
      ),
    ).toBe(true)

    expect(flightRouteCache.readChrome('/')).toBe(chrome)
    expect(flightRouteCache.readLeaf('/blog/b', '')).toEqual(pageB)
  })

  it('exports LayoutRouter components for live cache walk', () => {
    expect(typeof FlightDocument).toBe('function')
    expect(typeof FlightLayoutRouter).toBe('function')
  })
})
