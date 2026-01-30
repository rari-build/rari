export interface NavItem {
  label: string
  href?: string
  items?: NavItem[]
}

export const docsNavigation: NavItem[] = [
  {
    label: 'Getting Started',
    href: '/docs/getting-started',
    items: [
      { label: 'Metadata', href: '/docs/getting-started/metadata' },
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
    { label: 'Docs', href: '/docs' },
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

  findPath(docsNavigation, [{ label: 'Docs', href: '/docs' }])
  return breadcrumbs
}
