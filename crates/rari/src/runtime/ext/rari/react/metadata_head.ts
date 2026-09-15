/// <reference path="../core/types.d.ts" />

interface MetadataParams {
  readonly params: Readonly<Record<string, string>>
  readonly searchParams: Readonly<Record<string, string>>
}

interface IconDescriptor {
  readonly url?: string
  readonly rel?: string
  readonly type?: string
  readonly sizes?: string
}

interface OpenGraphImageDescriptor {
  readonly url?: string
  readonly width?: number
  readonly height?: number
  readonly alt?: string
}

type OpenGraphImage = string | OpenGraphImageDescriptor

type ReactApi = Readonly<NonNullable<typeof g.React>>

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function hasNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value !== ''
}

function elementChildren(props: Readonly<Record<string, unknown>> | undefined): unknown[] {
  const children = props?.children
  if (children == null) return []
  return Array.isArray(children) ? children : [children]
}

function createMeta(react: ReactApi, attrs: Readonly<Record<string, string>>): unknown {
  return react.createElement('meta', { ...attrs, 'data-rari-meta': '1' })
}

function createLink(react: ReactApi, attrs: Readonly<Record<string, string>>): unknown {
  return react.createElement('link', { ...attrs, 'data-rari-meta': '1' })
}

function iconDescriptors(value: unknown, defaultRel: string): IconDescriptor[] {
  if (value == null) return []
  if (typeof value === 'string') return [{ url: value, rel: defaultRel }]
  if (!Array.isArray(value) || value.length === 0) return []

  if (typeof value[0] === 'string') {
    const icons: IconDescriptor[] = []
    for (const item of value) {
      if (typeof item === 'string') icons.push({ url: item, rel: defaultRel })
    }
    return icons
  }

  const icons: IconDescriptor[] = []
  for (const item of value) {
    if (!isRecord(item)) continue
    icons.push({
      url: typeof item.url === 'string' ? item.url : undefined,
      rel: typeof item.rel === 'string' ? item.rel : defaultRel,
      type: typeof item.type === 'string' ? item.type : undefined,
      sizes: typeof item.sizes === 'string' ? item.sizes : undefined,
    })
  }
  return icons
}

function ogImageUrl(image: OpenGraphImage): string | undefined {
  if (typeof image === 'string') return image
  return image.url
}

function readOpenGraphImages(value: unknown): OpenGraphImage[] {
  if (!Array.isArray(value)) return []
  const images: OpenGraphImage[] = []
  for (const item of value) {
    if (typeof item === 'string') {
      images.push(item)
      continue
    }
    if (!isRecord(item)) continue
    images.push({
      url: typeof item.url === 'string' ? item.url : undefined,
      width: typeof item.width === 'number' ? item.width : undefined,
      height: typeof item.height === 'number' ? item.height : undefined,
      alt: typeof item.alt === 'string' ? item.alt : undefined,
    })
  }
  return images
}

export function buildMetadataHeadElements(metadata: unknown): unknown[] {
  if (!isRecord(metadata)) return []

  const react = g.React
  if (react == null || typeof react.createElement !== 'function') return []

  const elements: unknown[] = []
  const push = (node: unknown) => {
    if (node != null) elements.push(node)
  }

  push(createMeta(react, { charSet: 'UTF-8' }))

  const viewport =
    typeof metadata.viewport === 'string' && metadata.viewport !== ''
      ? metadata.viewport
      : 'width=device-width, initial-scale=1.0'
  push(createMeta(react, { name: 'viewport', content: viewport }))

  if (hasNonEmptyString(metadata.title)) {
    push(react.createElement('title', { 'data-rari-meta': '1' }, metadata.title))
  }

  if (hasNonEmptyString(metadata.description)) {
    push(createMeta(react, { name: 'description', content: metadata.description }))
  }

  if (Array.isArray(metadata.keywords) && metadata.keywords.length > 0) {
    const keywords = metadata.keywords.filter(
      (keyword): keyword is string => typeof keyword === 'string',
    )
    if (keywords.length > 0) {
      push(createMeta(react, { name: 'keywords', content: keywords.join(', ') }))
    }
  }

  if (isRecord(metadata.robots)) {
    const parts: string[] = []
    if (metadata.robots.index === false) parts.push('noindex')
    else if (metadata.robots.index === true) parts.push('index')
    if (metadata.robots.follow === false) parts.push('nofollow')
    else if (metadata.robots.follow === true) parts.push('follow')
    if (metadata.robots.nocache === true) parts.push('nocache')
    if (parts.length > 0) push(createMeta(react, { name: 'robots', content: parts.join(', ') }))
  }

  const alternates = isRecord(metadata.alternates) ? metadata.alternates : undefined
  const canonical =
    (typeof metadata.canonical === 'string' ? metadata.canonical : undefined) ??
    (typeof alternates?.canonical === 'string' ? alternates.canonical : undefined)
  if (hasNonEmptyString(canonical)) {
    push(createLink(react, { rel: 'canonical', href: canonical }))
  }

  if (isRecord(alternates?.types)) {
    for (const [type, href] of Object.entries(alternates.types)) {
      if (hasNonEmptyString(href)) push(createLink(react, { rel: 'alternate', type, href }))
    }
  }

  const icons = isRecord(metadata.icons) ? metadata.icons : undefined

  for (const icon of iconDescriptors(icons?.icon, 'icon')) {
    if (!hasNonEmptyString(icon.url)) continue
    const attrs: Record<string, string> = { rel: icon.rel ?? 'icon', href: icon.url }
    if (hasNonEmptyString(icon.type)) attrs.type = icon.type
    if (hasNonEmptyString(icon.sizes)) attrs.sizes = icon.sizes
    push(createLink(react, attrs))
  }

  for (const icon of iconDescriptors(icons?.apple, 'apple-touch-icon')) {
    if (!hasNonEmptyString(icon.url)) continue
    const attrs: Record<string, string> = {
      rel: icon.rel ?? 'apple-touch-icon',
      href: icon.url,
    }
    if (hasNonEmptyString(icon.type)) attrs.type = icon.type
    if (hasNonEmptyString(icon.sizes)) attrs.sizes = icon.sizes
    push(createLink(react, attrs))
  }

  for (const icon of iconDescriptors(icons?.other, 'icon')) {
    if (!hasNonEmptyString(icon.url)) continue
    const attrs: Record<string, string> = { rel: icon.rel ?? 'icon', href: icon.url }
    if (hasNonEmptyString(icon.type)) attrs.type = icon.type
    if (hasNonEmptyString(icon.sizes)) attrs.sizes = icon.sizes
    push(createLink(react, attrs))
  }

  const og = isRecord(metadata.openGraph) ? metadata.openGraph : undefined
  if (og != null) {
    if (hasNonEmptyString(og.title))
      push(createMeta(react, { property: 'og:title', content: og.title }))
    if (hasNonEmptyString(og.description))
      push(createMeta(react, { property: 'og:description', content: og.description }))
    if (hasNonEmptyString(og.url)) push(createMeta(react, { property: 'og:url', content: og.url }))
    if (hasNonEmptyString(og.siteName))
      push(createMeta(react, { property: 'og:site_name', content: og.siteName }))
    if (hasNonEmptyString(og.type))
      push(createMeta(react, { property: 'og:type', content: og.type }))
    for (const image of readOpenGraphImages(og.images)) {
      const url = ogImageUrl(image)
      if (hasNonEmptyString(url)) push(createMeta(react, { property: 'og:image', content: url }))
      if (typeof image !== 'string') {
        if (image.width != null)
          push(createMeta(react, { property: 'og:image:width', content: String(image.width) }))
        if (image.height != null)
          push(createMeta(react, { property: 'og:image:height', content: String(image.height) }))
        if (hasNonEmptyString(image.alt))
          push(createMeta(react, { property: 'og:image:alt', content: image.alt }))
      }
    }
  }

  const twitter = isRecord(metadata.twitter) ? metadata.twitter : undefined
  if (twitter != null) {
    if (hasNonEmptyString(twitter.card))
      push(createMeta(react, { name: 'twitter:card', content: twitter.card }))
    if (hasNonEmptyString(twitter.site))
      push(createMeta(react, { name: 'twitter:site', content: twitter.site }))
    if (hasNonEmptyString(twitter.creator))
      push(createMeta(react, { name: 'twitter:creator', content: twitter.creator }))
    if (hasNonEmptyString(twitter.title))
      push(createMeta(react, { name: 'twitter:title', content: twitter.title }))
    if (hasNonEmptyString(twitter.description))
      push(createMeta(react, { name: 'twitter:description', content: twitter.description }))
    if (Array.isArray(twitter.images)) {
      for (const image of twitter.images) {
        if (hasNonEmptyString(image))
          push(createMeta(react, { name: 'twitter:image', content: image }))
      }
    }
  }

  if (typeof metadata.themeColor === 'string') {
    push(createMeta(react, { name: 'theme-color', content: metadata.themeColor }))
  } else if (Array.isArray(metadata.themeColor)) {
    for (const entry of metadata.themeColor) {
      if (!isRecord(entry) || typeof entry.color !== 'string') continue
      const attrs: Record<string, string> = { name: 'theme-color', content: entry.color }
      if (hasNonEmptyString(entry.media)) attrs.media = entry.media
      push(createMeta(react, attrs))
    }
  }

  const appleWebApp = isRecord(metadata.appleWebApp) ? metadata.appleWebApp : undefined
  if (appleWebApp != null) {
    if (appleWebApp.capable === true) {
      push(createMeta(react, { name: 'apple-mobile-web-app-capable', content: 'yes' }))
    }
    if (hasNonEmptyString(appleWebApp.title)) {
      push(
        createMeta(react, {
          name: 'apple-mobile-web-app-title',
          content: appleWebApp.title,
        }),
      )
    }
    if (hasNonEmptyString(appleWebApp.statusBarStyle)) {
      push(
        createMeta(react, {
          name: 'apple-mobile-web-app-status-bar-style',
          content: appleWebApp.statusBarStyle,
        }),
      )
    }
  }

  return elements
}

function asDocumentElement(element: unknown): Record<string, unknown> | null {
  if (!isRecord(element)) return null
  if (element.type === 'html' || element.type === 'HTML') return element
  return null
}

export function injectMetadataIntoDocument(element: unknown, metadata: unknown): unknown {
  if (!isRecord(metadata) || Object.keys(metadata).length === 0) return element
  const react = g.React
  if (react == null || typeof react.createElement !== 'function') return element
  if (typeof react.cloneElement !== 'function') return element

  const documentElement = asDocumentElement(element)
  if (documentElement == null) return element

  const metaElements = buildMetadataHeadElements(metadata)
  if (metaElements.length === 0) return element

  const props = isRecord(documentElement.props) ? documentElement.props : {}
  const children = elementChildren(props)
  const headIndex = children.findIndex(
    child => isRecord(child) && (child.type === 'head' || child.type === 'HEAD'),
  )

  if (headIndex >= 0) {
    const head = children[headIndex]
    if (!isRecord(head)) return element
    const headProps = isRecord(head.props) ? head.props : {}
    const nextHeadChildren = [...elementChildren(headProps), ...metaElements]
    const nextHead = react.cloneElement(head, { ...headProps, children: nextHeadChildren })
    const nextChildren = [...children]
    nextChildren[headIndex] = nextHead
    return react.cloneElement(documentElement, { ...props, children: nextChildren })
  }

  const head = react.createElement('head', null, ...metaElements)
  return react.cloneElement(documentElement, { ...props, children: [head, ...children] })
}

g['~rari'] ??= {}
g['~rari'].injectMetadataIntoDocument = injectMetadataIntoDocument
g['~rari'].buildMetadataHeadElements = buildMetadataHeadElements

export type { MetadataParams }
