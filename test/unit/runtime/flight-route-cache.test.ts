import type { ReactElement, ReactNode } from 'react'
import {
  flightRouteCache,
  isLayoutSlot,
  segmentPathFromLayoutPath,
  segmentPathFromPathname,
} from '@rari/runtime/flight/route-cache'
import { createElement, isValidElement } from 'react'
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

describe('flightRouteCache (chrome / slot split)', () => {
  beforeEach(() => {
    flightRouteCache.clear()
  })

  it('parses segment paths', () => {
    expect(segmentPathFromPathname('/')).toEqual([])
    expect(segmentPathFromPathname('/blog/a')).toEqual(['blog', 'a'])
    expect(segmentPathFromLayoutPath('/blog/a')).toEqual(['blog', 'a'])
  })

  it('hollows nested stamps into slots and parks leaf data', () => {
    const page = createElement('main', null, 'page-a')
    const doc = htmlDoc(stamp('/', stamp('/blog', page)))
    flightRouteCache.set('/blog/a', '?x=1', doc)

    expect(flightRouteCache.hasRoute('/blog/a', '?x=1')).toBe(true)
    expect(flightRouteCache.hasRoute('/blog/a', '')).toBe(false)
    expect(flightRouteCache.readLeaf('/blog/a', '?x=1')).toEqual(page)

    const chrome = flightRouteCache.readChrome('/')
    expect(isValidElement(chrome)).toBe(true)
    if (!isValidElement(chrome)) return
    expect(isLayoutSlot(chrome)).toBe(true)
  })

  it('keeps parent chrome identity across leaf-only ingestSegment', () => {
    const pageA = createElement('main', null, 'a')
    const pageB = createElement('main', null, 'b')
    flightRouteCache.set('/blog/a', '', htmlDoc(stamp('/', stamp('/blog', pageA))))
    const chromeBefore = flightRouteCache.readChrome('/')

    expect(
      flightRouteCache.ingestSegment(
        createElement('rari-layout-reuse', { 'data-rari-layout-path': '/blog' }, pageB),
        '/blog/b',
        '',
      ),
    ).toBe(true)

    expect(flightRouteCache.readChrome('/')).toBe(chromeBefore)
    expect(flightRouteCache.readLeaf('/blog/b', '')).toEqual(pageB)
    expect(flightRouteCache.hasRoute('/blog/a', '')).toBe(true)
  })

  it('evictLeaf drops only that leaf page data', () => {
    flightRouteCache.set('/blog/a', '', htmlDoc(stamp('/blog', 'a')))
    flightRouteCache.set('/blog/b', '', htmlDoc(stamp('/blog', 'b')))
    flightRouteCache.evictLeaf('/blog/a', '')
    expect(flightRouteCache.hasRoute('/blog/a', '')).toBe(false)
    expect(flightRouteCache.hasRoute('/blog/b', '')).toBe(true)
  })

  it('invalidating a parent segment evicts nested pages', () => {
    flightRouteCache.set('/blog/a', '', htmlDoc(stamp('/blog', 'a')))
    flightRouteCache.set('/about', '', htmlDoc(stamp('/', 'about')))
    flightRouteCache.invalidate('/blog', '')
    expect(flightRouteCache.hasRoute('/blog/a', '')).toBe(false)
    expect(flightRouteCache.hasRoute('/about', '')).toBe(true)
  })

  it('invalidating root clears the shell', () => {
    flightRouteCache.set('/blog/a', '', htmlDoc(stamp('/', 'a')))
    flightRouteCache.invalidate('/', '')
    expect(flightRouteCache.getShell()).toBeNull()
    expect(flightRouteCache.hasRoute('/blog/a', '')).toBe(false)
  })

  it('getElement returns the hollow shell when the route exists', () => {
    flightRouteCache.set('/about', '', htmlDoc(stamp('/', 'about')))
    expect(flightRouteCache.getElement('/about', '')).toBe(flightRouteCache.getShell())
    expect(flightRouteCache.getElement('/missing', '')).toBeUndefined()
  })
})
