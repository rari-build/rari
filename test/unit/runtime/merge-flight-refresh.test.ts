import { mergeFlightRefresh } from '@rari/runtime/flight/merge-refresh'
import * as React from 'react'
import { describe, expect, it } from 'vite-plus/test'
import { castMock } from '../../helpers/mock-cast'

const CLIENT_REFERENCE = Symbol.for('react.client.reference')

function clientRef(id: string): React.ElementType {
  return castMock({
    $$typeof: CLIENT_REFERENCE,
    $$id: id,
    $$async: false,
  })
}

function expectElement(node: React.ReactNode): React.ReactElement<{
  'children'?: React.ReactNode
  'className'?: string
  'lang'?: string
  'href'?: string
  'name'?: string
  'content'?: string
  'charSet'?: string
  'data-theme'?: string
}> {
  if (
    !React.isValidElement<{
      'children'?: React.ReactNode
      'className'?: string
      'lang'?: string
      'href'?: string
      'name'?: string
      'content'?: string
      'charSet'?: string
      'data-theme'?: string
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

function childList(node: React.ReactElement<{ children?: React.ReactNode }>): React.ReactNode[] {
  const children = node.props.children
  if (Array.isArray(children))
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion React children array is widened
    return children as React.ReactNode[]
  if (children != null) return [children]
  return []
}

describe('mergeFlightRefresh', () => {
  it('uses the refresh tree when there is no current payload', () => {
    const refresh = React.createElement('div', { 'data-testid': 'page' }, 'fresh')

    expect(mergeFlightRefresh(null, refresh)).toBe(refresh)
  })

  it('adopts refreshed matching client trees so page ViewTransitions can enter', () => {
    const layoutType = clientRef('src/app/layout.tsx')
    const current = React.createElement(
      'section',
      null,
      React.createElement(layoutType, { key: '/' }, 'stale'),
    )
    const refresh = React.createElement(
      'section',
      null,
      React.createElement(clientRef('src/app/layout.tsx'), { key: '/' }, 'fresh'),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const mergedChild = expectElement(childList(merged)[0])

    expect(clientReferenceId(mergedChild.type)).toBe('src/app/layout.tsx')
    expect(mergedChild.type).not.toBe(layoutType)
    expect(String(mergedChild.key)).toContain('/')
    expect(mergedChild.props.children).toBe('fresh')
  })

  it('consumes nested reuse markers inside a matching client shell', () => {
    const shell = clientRef('src/app/providers.tsx')
    const current = React.createElement(
      shell,
      { key: 'providers' },
      React.createElement('main', null, 'home'),
    )
    const refresh = React.createElement(
      clientRef('src/app/providers.tsx'),
      { key: 'providers' },
      React.createElement('rari-layout-reuse', { 'data-rari-layout-path': '/' }, 'about'),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    expect(merged.type).toBe(shell)
    expect(expectElement(childList(merged)[0]).props.children).toBe('about')
  })

  it('replaces the tree when client component ids differ', () => {
    const current = React.createElement(clientRef('src/app/A.tsx'), { key: '/actions' })
    const refresh = React.createElement(clientRef('src/app/B.tsx'), { key: '/actions' })

    expect(mergeFlightRefresh(current, refresh)).toBe(refresh)
  })

  it('always takes the refreshed head for metadata', () => {
    const current = React.createElement(
      'html',
      null,
      React.createElement('head', null, React.createElement('title', null, 'Home')),
      React.createElement('body', null, 'home'),
    )
    const refresh = React.createElement(
      'html',
      null,
      React.createElement('head', null, React.createElement('title', null, 'About')),
      React.createElement('body', null, 'about'),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const [head, body] = childList(merged).map(child => expectElement(child))

    expect(head.type).toBe('head')
    expect(expectElement(childList(head)[0]).props.children).toBe('About')
    expect(body.type).toBe('body')
    expect(body.props.children).toBe('about')
  })

  it('adopts refreshed nested client layouts from the flight tree', () => {
    const rootLayout = clientRef('src/app/layout.tsx')
    const pageA = clientRef('src/app/page.tsx')
    const pageB = clientRef('src/app/about/page.tsx')

    const current = React.createElement(
      'body',
      null,
      React.createElement(
        rootLayout,
        { key: '/' },
        React.createElement(pageA, { key: 'page' }, 'home'),
      ),
    )
    const refresh = React.createElement(
      'body',
      null,
      React.createElement(
        clientRef('src/app/layout.tsx'),
        { key: '/' },
        React.createElement(pageB, { key: 'page' }, 'about'),
      ),
    )

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const mergedLayout = expectElement(childList(mergedBody)[0])
    expect(mergedLayout.type).not.toBe(rootLayout)
    expect(clientReferenceId(mergedLayout.type)).toBe('src/app/layout.tsx')

    const mergedPage = expectElement(childList(mergedLayout)[0])
    expect(clientReferenceId(mergedPage.type)).toBe('src/app/about/page.tsx')
    expect(mergedPage.props.children).toBe('about')
  })

  it('splices reuse markers into the previous layout shell', () => {
    const current = React.createElement(
      'body',
      null,
      React.createElement(
        'div',
        { className: 'shell' },
        React.createElement('nav', null, 'nav'),
        React.createElement('main', null, 'home'),
      ),
    )
    const refresh = React.createElement(
      'body',
      null,
      React.createElement('rari-layout-reuse', { 'data-rari-layout-path': '/' }, 'about'),
    )

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const shell = expectElement(childList(mergedBody)[0])
    expect(shell.props.className).toBe('shell')
    const kids = childList(shell)
    expect(expectElement(kids[0]).type).toBe('nav')
    expect(expectElement(kids[1]).type).toBe('main')
    expect(expectElement(kids[1]).props.children).toBe('about')
  })

  it('splices into multi-child body layouts via main without discarding siblings', () => {
    const current = React.createElement(
      'body',
      null,
      React.createElement('nav', null, 'nav'),
      React.createElement('main', null, 'home'),
      React.createElement('footer', null, 'footer'),
    )
    const refresh = React.createElement(
      'body',
      null,
      React.createElement('rari-layout-reuse', { 'data-rari-layout-path': '/' }, 'about'),
    )

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const kids = childList(mergedBody)
    expect(kids).toHaveLength(3)
    expect(expectElement(kids[0]).type).toBe('nav')
    expect(expectElement(kids[1]).type).toBe('main')
    expect(expectElement(kids[1]).props.children).toBe('about')
    expect(expectElement(kids[2]).type).toBe('footer')
  })

  it('does not splice into the first sibling when layout path misses and there is no main', () => {
    const current = React.createElement(
      'body',
      null,
      React.createElement('nav', null, 'nav'),
      React.createElement('div', { className: 'content' }, 'home'),
    )
    const refresh = React.createElement(
      'body',
      null,
      React.createElement('rari-layout-reuse', { 'data-rari-layout-path': '/missing' }, 'about'),
    )

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const kids = childList(mergedBody)
    expect(kids).toHaveLength(1)
    expect(kids[0]).toBe('about')
  })

  it('prefers nested main over preceding void or empty siblings', () => {
    const current = React.createElement(
      'body',
      null,
      React.createElement('img', { src: '/logo.png', alt: '' }),
      React.createElement('br'),
      React.createElement('div', null, React.createElement('main', null, 'home')),
    )
    const refresh = React.createElement(
      'body',
      null,
      React.createElement('rari-layout-reuse', { 'data-rari-layout-path': '/missing' }, 'about'),
    )

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const kids = childList(mergedBody)
    expect(kids).toHaveLength(3)
    expect(expectElement(kids[0]).type).toBe('img')
    expect(expectElement(kids[1]).type).toBe('br')
    const wrapper = expectElement(kids[2])
    expect(wrapper.type).toBe('div')
    expect(expectElement(childList(wrapper)[0]).props.children).toBe('about')
  })

  it('does not treat script or childless components as empty content slots', () => {
    function Sidebar(_props: Readonly<Record<string, never>>) {
      return null
    }

    const current = React.createElement(
      'body',
      null,
      React.createElement('script', { src: '/analytics.js' }),
      React.createElement(Sidebar, null),
      React.createElement('div', { className: 'slot' }),
    )
    const refresh = React.createElement(
      'body',
      null,
      React.createElement('rari-layout-reuse', { 'data-rari-layout-path': '/missing' }, 'about'),
    )

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const kids = childList(mergedBody)
    expect(kids).toHaveLength(3)
    expect(expectElement(kids[0]).type).toBe('script')
    expect(expectElement(kids[0]).props.children).toBeUndefined()
    expect(expectElement(kids[1]).type).toBe(Sidebar)
    expect(expectElement(kids[1]).props.children).toBeUndefined()
    const slot = expectElement(kids[2])
    expect(slot.type).toBe('div')
    expect(slot.props.className).toBe('slot')
    expect(slot.props.children).toBe('about')
  })

  it('does not treat empty select or title as content slots', () => {
    const current = React.createElement(
      'body',
      null,
      React.createElement('title', null),
      React.createElement('select', { name: 'locale' }),
      React.createElement('div', { className: 'page' }),
    )
    const refresh = React.createElement(
      'body',
      null,
      React.createElement('rari-layout-reuse', { 'data-rari-layout-path': '/missing' }, 'about'),
    )

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const kids = childList(mergedBody)
    expect(kids).toHaveLength(3)
    expect(expectElement(kids[0]).type).toBe('title')
    expect(expectElement(kids[0]).props.children).toBeUndefined()
    expect(expectElement(kids[1]).type).toBe('select')
    expect(expectElement(kids[1]).props.children).toBeUndefined()
    const page = expectElement(kids[2])
    expect(page.type).toBe('div')
    expect(page.props.className).toBe('page')
    expect(page.props.children).toBe('about')
  })

  it('clears shell children when reuse marker children are empty', () => {
    const current = React.createElement('main', { className: 'page' }, 'stale')
    const refresh = React.createElement('rari-layout-reuse', {
      'data-rari-layout-path': '/',
    })

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    expect(merged.type).toBe('main')
    expect(merged.props.className).toBe('page')
    expect(merged.props.children).toBeNull()
  })

  it('processes nested reuse markers outermost-inward', () => {
    const current = React.createElement(
      'body',
      null,
      React.createElement(
        'div',
        { 'data-rari-layout-path': '/', 'className': 'root' },
        React.createElement(
          'div',
          { 'data-rari-layout-path': '/blog', 'className': 'blog' },
          React.createElement('main', null, 'post-a'),
        ),
      ),
    )
    const refresh = React.createElement(
      'rari-layout-reuse',
      { 'data-rari-layout-path': '/' },
      React.createElement('rari-layout-reuse', { 'data-rari-layout-path': '/blog' }, 'post-b'),
    )

    const mergedBody = expectElement(mergeFlightRefresh(current, refresh))
    const root = expectElement(childList(mergedBody)[0])
    expect(root.props.className).toBe('root')
    const blog = expectElement(childList(root)[0])
    expect(blog.props.className).toBe('blog')
    expect(blog.props.children).toBe('post-b')
  })

  it('retains previous document attributes and head when merging reuse payloads', () => {
    const current = React.createElement(
      'html',
      { lang: 'en', className: 'doc' },
      React.createElement(
        'head',
        null,
        React.createElement('title', null, 'Home'),
        React.createElement('meta', { name: 'description', content: 'Home page' }),
        React.createElement('link', { rel: 'stylesheet', href: '/app.css' }),
      ),
      React.createElement('body', { className: 'body' }, React.createElement('main', null, 'home')),
    )
    const refresh = React.createElement(
      'html',
      null,
      React.createElement(
        'head',
        null,
        React.createElement('title', null, 'About'),
        React.createElement('meta', { name: 'description', content: 'About page' }),
      ),
      React.createElement(
        'body',
        null,
        React.createElement('rari-layout-reuse', { 'data-rari-layout-path': '/' }, 'about'),
      ),
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

  it('preserves existing viewport when the refreshed head has no replacement', () => {
    const current = React.createElement(
      'html',
      null,
      React.createElement(
        'head',
        null,
        React.createElement('meta', { charSet: 'utf-8' }),
        React.createElement('meta', { name: 'viewport', content: 'width=device-width' }),
        React.createElement('title', null, 'Home'),
        React.createElement('meta', { name: 'description', content: 'Home page' }),
      ),
      React.createElement('body', null, React.createElement('main', null, 'home')),
    )
    const refresh = React.createElement(
      'html',
      null,
      React.createElement(
        'head',
        null,
        React.createElement('title', null, 'About'),
        React.createElement('meta', { name: 'description', content: 'About page' }),
      ),
      React.createElement(
        'body',
        null,
        React.createElement('rari-layout-reuse', { 'data-rari-layout-path': '/' }, 'about'),
      ),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const [head] = childList(merged).map(child => expectElement(child))
    const headKids = childList(head).map(child => expectElement(child))
    const charset = headKids.find(child => child.type === 'meta' && child.props.charSet != null)
    const viewport = headKids.find(
      child => child.type === 'meta' && child.props.name === 'viewport',
    )
    const description = headKids.find(
      child => child.type === 'meta' && child.props.name === 'description',
    )
    expect(charset?.props.charSet).toBe('utf-8')
    expect(viewport?.props.content).toBe('width=device-width')
    expect(description?.props.content).toBe('About page')
    expect(headKids.find(child => child.type === 'title')?.props.children).toBe('About')
  })

  it('replaces viewport when the refreshed head supplies one inside a Fragment', () => {
    const current = React.createElement(
      'html',
      null,
      React.createElement(
        'head',
        null,
        React.createElement('meta', { name: 'viewport', content: 'width=device-width' }),
        React.createElement('title', null, 'Home'),
      ),
      React.createElement('body', null, React.createElement('main', null, 'home')),
    )
    const refresh = React.createElement(
      'html',
      null,
      React.createElement(
        'head',
        null,
        React.createElement(
          React.Fragment,
          null,
          React.createElement('meta', {
            name: 'viewport',
            content: 'width=device-width, initial-scale=1',
          }),
          React.createElement('title', null, 'About'),
        ),
      ),
      React.createElement(
        'body',
        null,
        React.createElement('rari-layout-reuse', { 'data-rari-layout-path': '/' }, 'about'),
      ),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const [head] = childList(merged).map(child => expectElement(child))
    const headKids = childList(head).map(child => expectElement(child))
    const viewports = headKids.filter(
      child => child.type === 'meta' && child.props.name === 'viewport',
    )
    expect(viewports).toHaveLength(1)
    expect(viewports[0]?.props.content).toBe('width=device-width, initial-scale=1')
    expect(headKids.find(child => child.type === 'title')?.props.children).toBe('About')
  })

  it('drops stale Fragment-wrapped title and meta from the current head', () => {
    const current = React.createElement(
      'html',
      null,
      React.createElement(
        'head',
        null,
        React.createElement(
          React.Fragment,
          null,
          React.createElement('title', null, 'Home'),
          React.createElement('meta', { name: 'description', content: 'Home page' }),
          React.createElement('link', { rel: 'stylesheet', href: '/app.css' }),
        ),
      ),
      React.createElement('body', null, React.createElement('main', null, 'home')),
    )
    const refresh = React.createElement(
      'html',
      null,
      React.createElement(
        'head',
        null,
        React.createElement('title', null, 'About'),
        React.createElement('meta', { name: 'description', content: 'About page' }),
      ),
      React.createElement(
        'body',
        null,
        React.createElement('rari-layout-reuse', { 'data-rari-layout-path': '/' }, 'about'),
      ),
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    const [head] = childList(merged).map(child => expectElement(child))
    const headKids = childList(head).map(child => expectElement(child))
    const description = headKids.find(
      child => child.type === 'meta' && child.props.name === 'description',
    )
    expect(headKids.filter(child => child.type === 'title')).toHaveLength(1)
    expect(headKids.find(child => child.type === 'title')?.props.children).toBe('About')
    expect(
      headKids.filter(child => child.type === 'meta' && child.props.name === 'description'),
    ).toHaveLength(1)
    expect(description?.props.content).toBe('About page')
    expect(headKids.find(child => child.type === 'link')?.props.href).toBe('/app.css')
  })

  it('merges a top-level document reuse marker into the previous html document', () => {
    const current = React.createElement(
      'html',
      { lang: 'en' },
      React.createElement('head', null, React.createElement('title', null, 'Home')),
      React.createElement(
        'body',
        null,
        React.createElement(
          'div',
          { className: 'shell' },
          React.createElement('main', null, 'home'),
        ),
      ),
    )
    const refresh = React.createElement(
      'rari-layout-reuse',
      { 'data-rari-layout-path': '/', 'data-rari-document-reuse': true },
      'about',
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    expect(merged.type).toBe('html')
    expect(merged.props.lang).toBe('en')
    const [, body] = childList(merged).map(child => expectElement(child))
    const shell = expectElement(childList(body)[0])
    expect(shell.props.className).toBe('shell')
    expect(expectElement(childList(shell)[0]).props.children).toBe('about')
  })

  it('retains a head-less html root when merging a document-reuse marker', () => {
    const current = React.createElement(
      'html',
      { 'lang': 'en', 'data-theme': 'dark' },
      React.createElement('body', { className: 'body' }, React.createElement('main', null, 'home')),
    )
    const refresh = React.createElement(
      'rari-layout-reuse',
      { 'data-rari-layout-path': '/', 'data-rari-document-reuse': true },
      'about',
    )

    const merged = expectElement(mergeFlightRefresh(current, refresh))
    expect(merged.type).toBe('html')
    expect(merged.props.lang).toBe('en')
    expect(merged.props['data-theme']).toBe('dark')
    const kids = childList(merged)
    expect(kids).toHaveLength(1)
    const body = expectElement(kids[0])
    expect(body.type).toBe('body')
    expect(body.props.className).toBe('body')
    expect(expectElement(childList(body)[0]).props.children).toBe('about')
  })

  it('unwraps reuse markers when there is no current shell to splice into', () => {
    const refresh = React.createElement(
      'div',
      null,
      React.createElement(
        'rari-layout-reuse',
        { 'data-rari-layout-path': '/docs' },
        React.createElement('main', null, 'fresh'),
      ),
    )

    const merged = expectElement(mergeFlightRefresh(null, refresh))
    expect(merged.type).toBe('div')
    const main = expectElement(childList(merged)[0])
    expect(main.type).toBe('main')
    expect(main.props.children).toBe('fresh')
  })
})
