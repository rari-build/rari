import type { ElementType, ReactElement, ReactNode } from 'react'
import { mergeFlightRefresh, unwrapFulfilledFlightNode } from '@rari/runtime/flight/merge-refresh'
import { createElement, Fragment, isValidElement, Suspense } from 'react'
import { describe, expect, it } from 'vite-plus/test'
import { fulfilledFlightNode } from '../../helpers/flight-thenable'
import { castMock } from '../../helpers/mock-cast'

const CLIENT_REFERENCE = Symbol.for('react.client.reference')

function clientRef(id: string): ElementType {
  return castMock({
    $$typeof: CLIENT_REFERENCE,
    $$id: id,
    $$async: false,
  })
}

function expectElement(node: ReactNode): ReactElement<{
  'children'?: ReactNode
  'className'?: string
  'lang'?: string
  'href'?: string
  'name'?: string
  'content'?: string
  'charSet'?: string
  'role'?: string
  'data-theme'?: string
  'data-rari-layout-path'?: string
}> {
  if (
    !isValidElement<{
      'children'?: ReactNode
      'className'?: string
      'lang'?: string
      'href'?: string
      'name'?: string
      'content'?: string
      'charSet'?: string
      'role'?: string
      'data-theme'?: string
      'data-rari-layout-path'?: string
    }>(node)
  )
    throw new Error('Expected React element')

  return node
}

function clientReferenceId(type: unknown): string | undefined {
  if (typeof type !== 'function' && (typeof type !== 'object' || type == null)) return undefined

  const id: unknown = Reflect.get(type, '$$id')
  return typeof id === 'string' ? id : undefined
}

function childList(node: ReactElement<{ children?: ReactNode }>): ReactNode[] {
  const { children } = node.props
  // oxlint-disable-next-line typescript/no-unsafe-return
  if (Array.isArray(children)) return children
  if (children != null) return [children]
  return []
}

function stamp(path: string, child: ReactNode): ReactElement {
  return createElement('div', { 'data-rari-layout-path': path }, child)
}

function reuse(path: string, child?: ReactNode): ReactElement {
  return createElement('rari-layout-reuse', { 'data-rari-layout-path': path }, child)
}

describe('mergeFlightRefresh', () => {
  it('unwraps fulfilled Flight thenables that resolve to null/boolean holes', () => {
    expect(unwrapFulfilledFlightNode(fulfilledFlightNode(null))).toBeNull()
    expect(unwrapFulfilledFlightNode(fulfilledFlightNode(false))).toBe(false)
    expect(unwrapFulfilledFlightNode(fulfilledFlightNode(true))).toBe(true)
  })

  it('uses the refresh tree when there is no current payload', () => {
    const refresh = createElement('div', { 'data-testid': 'page' }, 'fresh')
    expect(mergeFlightRefresh(null, refresh)).toBe(refresh)
  })

  it('adopts refreshed matching client trees so page ViewTransitions can enter', () => {
    const layoutType = clientRef('src/app/layout.tsx')
    const current = createElement('section', null, createElement(layoutType, { key: '/' }, 'stale'))
    const refresh = createElement(
      'section',
      null,
      createElement(clientRef('src/app/layout.tsx'), { key: '/' }, 'fresh'),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const mergedChild = expectElement(childList(merged)[0])

    expect(clientReferenceId(mergedChild.type)).toBe('src/app/layout.tsx')
    expect(mergedChild.type).not.toBe(layoutType)
    expect(String(mergedChild.key)).toContain('/')
    expect(mergedChild.props.children).toBe('fresh')
  })

  it('replaces the tree when client component ids differ', () => {
    const current = createElement(clientRef('src/app/A.tsx'), { key: '/actions' })
    const refresh = createElement(clientRef('src/app/B.tsx'), { key: '/actions' })

    expect(mergeFlightRefresh(current, refresh)).toBe(refresh)
  })

  it('always takes the refreshed head for metadata', () => {
    const current = createElement(
      'html',
      null,
      createElement('head', null, createElement('title', null, 'Home')),
      createElement('body', null, 'home'),
    )
    const refresh = createElement(
      'html',
      null,
      createElement('head', null, createElement('title', null, 'About')),
      createElement('body', null, 'about'),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const [head, body] = childList(merged).map(child => expectElement(child))

    expect(head.type).toBe('head')
    expect(expectElement(childList(head)[0]).props.children).toBe('About')
    expect(body.type).toBe('body')
    expect(body.props.children).toBe('about')
  })

  it('splices into a stamped path slot beside HTML chrome siblings', () => {
    const current = createElement(
      'body',
      null,
      createElement('nav', null, 'nav'),
      stamp('/', 'home'),
      createElement('footer', null, 'footer'),
    )
    const refresh = createElement('body', null, reuse('/', 'about'))

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const kids = childList(mergedBody)
    expect(kids).toHaveLength(3)
    expect(expectElement(kids[0]).type).toBe('nav')
    expect(expectElement(kids[1]).props['data-rari-layout-path']).toBe('/')
    expect(expectElement(kids[1]).props.children).toBe('about')
    expect(expectElement(kids[2]).type).toBe('footer')
  })

  it('keeps the previous tree when the layout path stamp is missing', () => {
    const current = createElement(
      'body',
      null,
      createElement('nav', null, 'nav'),
      createElement('main', null, 'home'),
      createElement('footer', null, 'footer'),
    )
    const refresh = createElement('body', null, reuse('/', 'about'))

    expect(mergeFlightRefresh(current, refresh)).toBe(current)
  })

  it('keeps the previous html document identity when the layout path stamp is missing', () => {
    const current = createElement(
      'html',
      { lang: 'en' },
      createElement('head', null),
      createElement(
        'body',
        null,
        createElement('nav', null, 'nav'),
        createElement('main', null, 'home'),
      ),
    )
    const refresh = reuse('/', 'about')

    expect(mergeFlightRefresh(current, refresh)).toBe(current)
  })

  it('splices when the current element itself is the stamped host', () => {
    const shell = clientRef('src/app/providers.tsx')
    const current = createElement(shell, { key: 'providers' }, stamp('/', 'home'))
    const refresh = createElement(
      clientRef('src/app/providers.tsx'),
      { key: 'providers' },
      reuse('/', 'about'),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    expect(merged.type).toBe(shell)
    const slot = expectElement(childList(merged)[0])
    expect(slot.props['data-rari-layout-path']).toBe('/')
    expect(slot.props.children).toBe('about')
  })

  it('clears stamped slot children when reuse marker children are empty', () => {
    const current = stamp('/', createElement('main', { className: 'page' }, 'stale'))
    const refresh = reuse('/')

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    expect(merged.props['data-rari-layout-path']).toBe('/')
    expect(merged.props.children).toBeNull()
  })

  it('processes nested reuse markers outermost-inward', () => {
    const current = createElement(
      'body',
      null,
      createElement(
        'div',
        { 'data-rari-layout-path': '/', 'className': 'root' },
        createElement(
          'div',
          { 'data-rari-layout-path': '/blog', 'className': 'blog' },
          createElement('main', null, 'post-a'),
        ),
      ),
    )
    const refresh = reuse('/', reuse('/blog', 'post-b'))

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const root = expectElement(childList(mergedBody)[0])
    expect(root.props.className).toBe('root')
    const blog = expectElement(childList(root)[0])
    expect(blog.props.className).toBe('blog')
    expect(blog.props.children).toBe('post-b')
  })

  it('consumes nested reuse markers separated by a matching client wrapper', () => {
    const current = createElement(
      'body',
      null,
      createElement(
        'div',
        { 'data-rari-layout-path': '/', 'className': 'root' },
        createElement(
          clientRef('src/app/blog/layout.tsx'),
          { key: '/blog' },
          createElement(
            'div',
            { 'data-rari-layout-path': '/blog', 'className': 'blog' },
            createElement('main', null, 'post-a'),
          ),
        ),
      ),
    )
    const refresh = reuse(
      '/',
      createElement(
        clientRef('src/app/blog/layout.tsx'),
        { key: '/blog' },
        reuse('/blog', 'post-b'),
      ),
    )

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const root = expectElement(childList(mergedBody)[0])
    expect(root.props.className).toBe('root')
    const client = expectElement(childList(root)[0])
    expect(clientReferenceId(client.type)).toBe('src/app/blog/layout.tsx')
    const blog = expectElement(childList(client)[0])
    expect(blog.props.className).toBe('blog')
    expect(blog.props.children).toBe('post-b')
  })

  it('soft-navs a document reuse marker into a stamped body slot', () => {
    const current = createElement(
      'html',
      { lang: 'en' },
      createElement('head', null),
      createElement(
        'body',
        { className: 'bg-gray-950' },
        createElement('nav', null, 'navbar'),
        stamp('/', createElement('main', null, 'home')),
        createElement(Suspense, { fallback: null }, createElement('footer', null, 'footer')),
      ),
    )
    const refresh = reuse('/', 'posts')

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const body = expectElement(childList(merged)[1])
    const kids = childList(body)
    expect(kids).toHaveLength(3)
    expect(expectElement(kids[0]).type).toBe('nav')
    expect(expectElement(kids[1]).props['data-rari-layout-path']).toBe('/')
    expect(expectElement(kids[1]).props.children).toBe('posts')
    expect(expectElement(kids[2]).type).toBe(Suspense)
  })

  it('soft-navs when body children are one fulfilled thenable containing a stamp', () => {
    const current = createElement(
      'html',
      { lang: 'en' },
      createElement('head', null),
      createElement(
        'body',
        { className: 'bg-gray-950' },
        fulfilledFlightNode([
          createElement('nav', { key: 'nav' }, 'navbar'),
          stamp('/', createElement('main', { key: 'main' }, 'home')),
          createElement(
            Suspense,
            { key: 'footer', fallback: null },
            createElement('footer', null, 'footer'),
          ),
        ]),
      ),
    )
    const refresh = reuse('/', 'posts')

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const body = expectElement(childList(merged)[1])
    const kids = childList(body)
    expect(kids).toHaveLength(1)
    const slot = expectElement(kids[0])
    expect(slot.type).toBe(Fragment)
    const slotKids = childList(slot)
    expect(slotKids).toHaveLength(3)
    expect(expectElement(slotKids[0]).type).toBe('nav')
    expect(expectElement(slotKids[1]).props['data-rari-layout-path']).toBe('/')
    expect(expectElement(slotKids[1]).props.children).toBe('posts')
    expect(expectElement(slotKids[2]).type).toBe(Suspense)
  })

  it('preserves a Fragment chrome slot while replacing a stamped page inside it', () => {
    const current = createElement(
      'body',
      null,
      createElement(
        Fragment,
        null,
        createElement('nav', null, 'nav'),
        stamp('/', createElement('main', null, 'home')),
        createElement('footer', null, 'footer'),
      ),
    )
    const refresh = createElement('body', null, reuse('/', 'about'))

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const fragment = expectElement(childList(mergedBody)[0])
    expect(fragment.type).toBe(Fragment)
    const slotKids = childList(fragment)
    expect(slotKids).toHaveLength(3)
    expect(expectElement(slotKids[0]).type).toBe('nav')
    expect(expectElement(slotKids[1]).props.children).toBe('about')
    expect(expectElement(slotKids[2]).type).toBe('footer')
  })

  it('splices a nested layout path inside thenable-wrapped chrome without picking the wrong slot', () => {
    const current = createElement(
      'body',
      null,
      createElement(
        'div',
        { className: 'shell' },
        createElement('nav', null, 'nav'),
        fulfilledFlightNode(
          createElement(
            'div',
            { 'data-rari-layout-path': '/sidebar', 'className': 'side' },
            'side-a',
          ),
        ),
        createElement('div', { 'data-rari-layout-path': '/blog', 'className': 'blog' }, 'post-a'),
        createElement('footer', null, 'footer'),
      ),
    )
    const refresh = createElement('body', null, reuse('/sidebar', 'side-b'))

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const shell = expectElement(childList(mergedBody)[0])
    expect(shell.props.className).toBe('shell')
    const kids = childList(shell)
    expect(kids).toHaveLength(4)
    expect(expectElement(kids[0]).type).toBe('nav')
    expect(expectElement(kids[1]).props.className).toBe('side')
    expect(expectElement(kids[1]).props.children).toBe('side-b')
    expect(expectElement(kids[2]).props.className).toBe('blog')
    expect(expectElement(kids[2]).props.children).toBe('post-a')
    expect(expectElement(kids[3]).type).toBe('footer')
  })

  it('soft-navs Providers shell with stamped page slot on document reuse', () => {
    const providers = clientRef('src/providers.tsx')
    const navbar = clientRef('src/components/ui/Navbar.tsx')
    const current = createElement(
      'html',
      { lang: 'en' },
      createElement('head', null),
      createElement(
        'body',
        null,
        createElement(
          providers,
          { key: 'providers' },
          createElement(navbar, { key: 'nav' }),
          stamp('/', fulfilledFlightNode(createElement('main', null, 'home'))),
          createElement(Suspense, { fallback: null }, createElement('footer', null, 'footer')),
        ),
      ),
    )
    const refresh = reuse('/', 'posts')

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const body = expectElement(childList(merged)[1])
    const shell = expectElement(childList(body)[0])
    expect(clientReferenceId(shell.type)).toBe('src/providers.tsx')
    const kids = childList(shell)
    expect(kids).toHaveLength(3)
    expect(clientReferenceId(expectElement(kids[0]).type)).toBe('src/components/ui/Navbar.tsx')
    expect(expectElement(kids[1]).props['data-rari-layout-path']).toBe('/')
    expect(expectElement(kids[1]).props.children).toBe('posts')
    expect(expectElement(kids[2]).type).toBe(Suspense)
  })

  it('retains previous document attributes and head when merging reuse payloads', () => {
    const current = createElement(
      'html',
      { lang: 'en', className: 'doc' },
      createElement(
        'head',
        null,
        createElement('title', null, 'Home'),
        createElement('meta', { name: 'description', content: 'Home page' }),
        createElement('link', { rel: 'stylesheet', href: '/app.css' }),
      ),
      createElement('body', { className: 'body' }, stamp('/', createElement('main', null, 'home'))),
    )
    const refresh = createElement(
      'html',
      null,
      createElement(
        'head',
        null,
        createElement('title', null, 'About'),
        createElement('meta', { name: 'description', content: 'About page' }),
      ),
      createElement('body', null, reuse('/', 'about')),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    expect(merged.type).toBe('html')
    expect(merged.props.lang).toBe('en')
    expect(merged.props.className).toBe('doc')

    const [head, body] = childList(merged).map(child => expectElement(child))
    expect(head.type).toBe('head')
    const headKids = childList(head).map(child => expectElement(child))
    const title = headKids.find(child => child.type === 'title')
    const description = headKids.find(
      child => child.type === 'meta' && child.props.name === 'description',
    )
    const stylesheet = headKids.find(child => child.type === 'link')
    expect(title?.props.children).toBe('About')
    expect(description?.props.content).toBe('About page')
    expect(headKids.filter(child => child.type === 'meta')).toHaveLength(1)
    expect(stylesheet?.props.href).toBe('/app.css')

    expect(body.props.className).toBe('body')
    expect(expectElement(childList(body)[0]).props.children).toBe('about')
  })

  it('preserves title and description when the refreshed head is empty', () => {
    const current = createElement(
      'html',
      null,
      createElement(
        'head',
        null,
        createElement('title', null, 'Home'),
        createElement('meta', { name: 'description', content: 'Home page' }),
      ),
      createElement('body', null, stamp('/', 'home')),
    )
    const refresh = createElement(
      'html',
      null,
      createElement('head', null),
      createElement('body', null, reuse('/', 'about')),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const [head, body] = childList(merged).map(child => expectElement(child))
    const headKids = childList(head).map(child => expectElement(child))
    expect(headKids.find(child => child.type === 'title')?.props.children).toBe('Home')
    expect(
      headKids.find(child => child.type === 'meta' && child.props.name === 'description')?.props
        .content,
    ).toBe('Home page')
    expect(expectElement(childList(body)[0]).props.children).toBe('about')
  })

  it('preserves existing viewport when the refreshed head has no replacement', () => {
    const current = createElement(
      'html',
      null,
      createElement(
        'head',
        null,
        createElement('meta', { name: 'viewport', content: 'width=device-width' }),
        createElement('title', null, 'Home'),
      ),
      createElement('body', null, stamp('/', 'home')),
    )
    const refresh = createElement(
      'html',
      null,
      createElement('head', null, createElement('title', null, 'About')),
      createElement('body', null, reuse('/', 'about')),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const head = expectElement(childList(merged)[0])
    const headKids = childList(head).map(child => expectElement(child))
    expect(
      headKids.find(child => child.type === 'meta' && child.props.name === 'viewport')?.props
        .content,
    ).toBe('width=device-width')
    expect(headKids.find(child => child.type === 'title')?.props.children).toBe('About')
  })

  it('replaces viewport when the refreshed head supplies one inside a Fragment', () => {
    const current = createElement(
      'html',
      null,
      createElement(
        'head',
        null,
        createElement('meta', { name: 'viewport', content: 'width=device-width' }),
      ),
      createElement('body', null, stamp('/', 'home')),
    )
    const refresh = createElement(
      'html',
      null,
      createElement(
        'head',
        null,
        createElement(
          Fragment,
          null,
          createElement('meta', { name: 'viewport', content: 'width=390' }),
        ),
      ),
      createElement('body', null, reuse('/', 'about')),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const head = expectElement(childList(merged)[0])
    const headKids = childList(head).map(child => expectElement(child))
    expect(
      headKids.find(child => child.type === 'meta' && child.props.name === 'viewport')?.props
        .content,
    ).toBe('width=390')
  })

  it('drops stale Fragment-wrapped title and meta from the current head', () => {
    const current = createElement(
      'html',
      null,
      createElement(
        'head',
        null,
        createElement(
          Fragment,
          null,
          createElement('title', null, 'Home'),
          createElement('meta', { name: 'description', content: 'Home page' }),
        ),
        createElement('link', { rel: 'stylesheet', href: '/app.css' }),
      ),
      createElement('body', null, stamp('/', 'home')),
    )
    const refresh = createElement(
      'html',
      null,
      createElement(
        'head',
        null,
        createElement('title', null, 'About'),
        createElement('meta', { name: 'description', content: 'About page' }),
      ),
      createElement('body', null, reuse('/', 'about')),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const head = expectElement(childList(merged)[0])
    const headKids = childList(head).map(child => expectElement(child))
    expect(headKids.filter(child => child.type === 'title')).toHaveLength(1)
    expect(headKids.find(child => child.type === 'title')?.props.children).toBe('About')
    expect(headKids.filter(child => child.type === 'meta')).toHaveLength(1)
    expect(headKids.find(child => child.type === 'link')?.props.href).toBe('/app.css')
  })

  it('merges a top-level document reuse marker into the previous html document', () => {
    const current = createElement(
      'html',
      { lang: 'en' },
      createElement('head', null, createElement('title', null, 'Home')),
      createElement('body', null, stamp('/', 'home')),
    )
    const refresh = reuse('/', 'about')

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    expect(merged.type).toBe('html')
    expect(merged.props.lang).toBe('en')
    const [, body] = childList(merged).map(child => expectElement(child))
    expect(expectElement(childList(body)[0]).props.children).toBe('about')
  })

  it('retains a head-less html root when merging a document-reuse marker', () => {
    const current = createElement('html', null, createElement('body', null, stamp('/', 'home')))
    const refresh = reuse('/', 'about')

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    expect(merged.type).toBe('html')
    const body = expectElement(childList(merged)[0])
    expect(body.type).toBe('body')
    expect(expectElement(childList(body)[0]).props.children).toBe('about')
  })

  it('unwraps reuse markers when there is no current shell to splice into', () => {
    expect(mergeFlightRefresh(null, reuse('/', 'about'))).toBe('about')
  })

  it('flattens multi-child reuse markers beside siblings when unwrapping', () => {
    const refresh = createElement(
      'div',
      null,
      createElement('aside', null, 'side'),
      reuse('/', createElement('main', null, 'page')),
    )
    const merged = expectElement(mergeFlightRefresh(null, refresh))
    const kids = childList(merged)
    expect(kids).toHaveLength(2)
    expect(expectElement(kids[0]).type).toBe('aside')
    expect(expectElement(kids[1]).type).toBe('main')
  })

  it('keeps nested-array key scopes distinct through merge flatten', () => {
    const current = createElement('div', null, [
      [createElement('span', { key: 'a' }, 'a')],
      [createElement('span', { key: 'b' }, 'b')],
    ])
    const refresh = createElement('div', null, [
      [createElement('span', { key: 'a' }, 'A')],
      [createElement('span', { key: 'b' }, 'B')],
    ])

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const kids = childList(merged).map(child => expectElement(child))
    expect(kids).toHaveLength(2)
    expect(String(kids[0].key)).not.toBe(String(kids[1].key))
    expect(kids[0].props.children).toBe('A')
    expect(kids[1].props.children).toBe('B')
  })

  it('replaces matched page content with empty children instead of retaining stale kids', () => {
    const current = createElement('main', null, createElement('p', null, 'old'))
    const refresh = createElement('main', null)

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    expect(childList(merged)).toEqual([])
  })

  it('merges matching client shells pairwise so ViewTransitions still adopt refresh identity', () => {
    const layoutType = clientRef('src/app/layout.tsx')
    const current = createElement(
      layoutType,
      { key: '/' },
      createElement('nav', null, 'nav'),
      createElement('main', null, 'stale'),
    )
    const refresh = createElement(
      clientRef('src/app/layout.tsx'),
      { key: '/' },
      createElement('nav', null, 'nav'),
      createElement('main', null, 'fresh'),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    expect(clientReferenceId(merged.type)).toBe('src/app/layout.tsx')
    expect(merged.type).not.toBe(layoutType)
    const kids = childList(merged)
    expect(expectElement(kids[0]).type).toBe('nav')
    expect(expectElement(kids[1]).type).toBe('main')
    expect(expectElement(kids[1]).props.children).toBe('fresh')
  })

  it('replaces a stamped ClientPage shell with the refreshed page', () => {
    const clientPage = clientRef('src/app/page.tsx')
    const current = createElement(
      'body',
      null,
      createElement('header', null, 'site'),
      stamp('/', createElement(clientPage, { key: 'page' })),
    )
    const refresh = createElement(
      'body',
      null,
      reuse('/', createElement(clientRef('src/app/about.tsx'), { key: 'about' }, 'about')),
    )

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const kids = childList(mergedBody).map(child => expectElement(child))
    expect(kids).toHaveLength(2)
    expect(kids[0].type).toBe('header')
    expect(kids[1].props['data-rari-layout-path']).toBe('/')
    expect(clientReferenceId(expectElement(childList(kids[1])[0]).type)).toBe('src/app/about.tsx')
    expect(expectElement(childList(kids[1])[0]).props.children).toBe('about')
  })
})
