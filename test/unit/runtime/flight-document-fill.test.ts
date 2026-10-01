import type { ReactElement, ReactNode } from 'react'
import { applySoftNavFlightPatch } from '@rari/runtime/flight/apply-flight-patch'
import {
  FlightLayoutRouter,
  renderFlightDocument,
  renderFlightLayoutRouter,
} from '@rari/runtime/flight/layout-router'
import {
  flightRouteCache,
  isLayoutSlot,
  LAYOUT_SLOT_ELEMENT,
} from '@rari/runtime/flight/route-cache'
import { createElement, isValidElement } from 'react'
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test'

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
    createElement('head', null),
    createElement(
      'body',
      null,
      createElement('nav', null, 'SiteNav'),
      createElement('main', null, bodyChild),
    ),
  )
}

describe('flightDocument soft-nav fill', () => {
  beforeEach(() => {
    flightRouteCache.clear()
    vi.stubGlobal('window', {
      location: { pathname: '/about', search: '', href: 'http://localhost/about' },
    })
  })

  it('soft-nav leaf is readable and deepest at root', () => {
    const previous = htmlDoc(stamp('/', createElement('section', null, 'home')))
    flightRouteCache.set('/', '', previous)

    const patched = applySoftNavFlightPatch({
      previousDocument: previous,
      refresh: reuse('/', createElement('section', null, 'about')),
      fromPathname: '/',
      toPathname: '/about',
      fromSearch: '',
      search: '',
    })

    expect(patched.kind).toBe('merged')
    expect(flightRouteCache.hasRoute('/about', '')).toBe(true)
    expect(flightRouteCache.isDeepestLayout('/', '/about')).toBe(true)
    expect(flightRouteCache.readSegmentSnapshot('/', { pathname: '/about', search: '' })).toEqual(
      createElement('section', null, 'about'),
    )

    const leaf = flightRouteCache.readLeaf('/about', '')
    expect(isValidElement(leaf) && isLayoutSlot(leaf)).toBe(false)
  })

  it('reingesting hollow shell is a no-op (does not park slots as leaves)', () => {
    const previous = htmlDoc(stamp('/', createElement('section', null, 'home')))
    flightRouteCache.set('/', '', previous)
    const shell = flightRouteCache.getShell()
    expect(shell).not.toBeNull()

    flightRouteCache.clear()
    if (shell != null) flightRouteCache.set('/about', '', shell)

    expect(flightRouteCache.getShell()).toBeNull()
    expect(flightRouteCache.hasRoute('/about', '')).toBe(false)
  })

  it('flightLayoutRouter does not emit host layout slots for a healthy cache', () => {
    const previous = htmlDoc(stamp('/', createElement('section', null, 'home')))
    flightRouteCache.set('/', '', previous)
    applySoftNavFlightPatch({
      previousDocument: previous,
      refresh: reuse('/', createElement('section', null, 'about')),
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
    expect(rendered.key).toBe('/about')
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion
    expect(rendered.props).toEqual({ children: createElement('section', null, 'about') })
    expect(String(FlightLayoutRouter)).not.toBe(LAYOUT_SLOT_ELEMENT)
  })

  it('flightDocument returns filled tree when ok', () => {
    const previous = htmlDoc(stamp('/', createElement('section', null, 'home')))
    flightRouteCache.set('/', '', previous)
    const patched = applySoftNavFlightPatch({
      previousDocument: previous,
      refresh: reuse('/', createElement('section', null, 'about')),
      fromPathname: '/',
      toPathname: '/about',
      fromSearch: '',
      search: '',
    })
    expect(patched.kind).toBe('merged')

    const shell = flightRouteCache.getShell()
    const ok = flightRouteCache.hasRoute('/about', '')
    expect(shell).not.toBeNull()
    expect(ok).toBe(true)
  })

  it('flightDocument does not paint a previous-route fallback when shell exists', () => {
    const previous = htmlDoc(stamp('/', createElement('section', { 'data-page': 'home' }, 'home')))
    flightRouteCache.set('/', '', previous)

    const homeFallback = previous
    const rendered = renderFlightDocument({
      fallback: homeFallback,
      pathname: '/missing',
      search: '',
    })

    expect(isValidElement(rendered)).toBe(true)
    if (!isValidElement(rendered)) return
    const html = JSON.stringify(rendered)
    expect(html).not.toContain('data-page')
    expect(html).toContain('SiteNav')
  })
})
