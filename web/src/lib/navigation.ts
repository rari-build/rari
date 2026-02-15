interface NavItem {
  label: string
  href?: string
  items?: NavItem[]
}

export const docsNavigation: NavItem[] = [
  {
    label: 'Getting Started',
    href: '/docs/getting-started',
    items: [
      { label: 'Routing', href: '/docs/getting-started/routing' },
      { label: 'Metadata', href: '/docs/getting-started/metadata' },
      { label: 'Deploying', href: '/docs/getting-started/deploying' },
    ],
  },
  {
    label: 'API Reference',
    href: '/docs/api-reference',
    items: [
      {
        label: 'Components',
        href: '/docs/api-reference/components',
        items: [
          { label: 'Image', href: '/docs/api-reference/components/image' },
          { label: 'ImageResponse', href: '/docs/api-reference/components/image-response' },
        ],
      },
    ],
  },
]

export function getBreadcrumbs(path: string): Array<{ label: string, href?: string }> {
  const breadcrumbs: Array<{ label: string, href?: string }> = [
    { label: 'Docs', href: '/docs/getting-started' },
  ]

  function findPath(items: NavItem[], currentPath: Array<{ label: string, href?: string }>): boolean {
    for (const item of items) {
      const newPath = [...currentPath, { label: item.label, href: item.href }]

      if (item.href === path) {
        breadcrumbs.push(...currentPath.slice(1))
        breadcrumbs.push({ label: item.label })
        return true
      }

      if (item.items && findPath(item.items, newPath))
        return true
    }

    return false
  }

  findPath(docsNavigation, [{ label: 'Docs', href: '/docs/getting-started' }])
  return breadcrumbs
}
