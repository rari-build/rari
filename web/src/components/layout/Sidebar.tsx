'use client'

import type { Dispatch, ReactNode, SetStateAction } from 'react'
import type { NavItem } from '@/lib/content/docs-navigation'
import { usePathname } from 'rari/router'
import { useEffect, useMemo, useRef, useState } from 'react'
import { docsNavigation } from '@/lib/content/docs-navigation'
import Bluesky from '../icons/Bluesky'
import ChevronRight from '../icons/ChevronRight'
import Close from '../icons/Close'
import Discord from '../icons/Discord'
import Github from '../icons/Github'
import Heart from '../icons/Heart'
import Menu from '../icons/Menu'
import Rari from '../icons/Rari'
import SearchBar from '../search/SearchBar'
import ThemeSwitcher from './ThemeSwitcher'

interface TopNavItem {
  readonly href: string
  readonly label: string
  readonly id?: string
  readonly external?: boolean
  readonly items?: readonly { readonly href: string; readonly label: string }[]
}

interface SidebarProps {
  readonly version: ReactNode
}

const navigation: readonly TopNavItem[] = [
  { href: '/docs/getting-started', label: 'Docs', id: 'docs' },
  {
    href: '/enterprise',
    label: 'Enterprise',
    id: 'enterprise',
    items: [{ href: '/enterprise/sponsors', label: 'Sponsors' }],
  },
  { href: '/blog', label: 'Blog', id: 'blog' },
  {
    href: 'https://github.com/sponsors/skiniks',
    label: 'Become a Sponsor',
    id: 'sponsor',
    external: true,
  },
]

const linkBase = 'rounded-md text-sm transition-all duration-200 relative overflow-hidden group'
const linkIdle = 'text-fg-muted hover:bg-hover hover:text-fg'
const linkActive = 'bg-linear-to-r from-accent/20 to-accent-hover/20 text-fg'
const hoverWash =
  'absolute inset-0 bg-linear-to-r from-accent/10 to-accent-hover/10 opacity-0 group-hover:opacity-100 transition-opacity duration-300'

function navKey(item: Pick<NavItem, 'href' | 'label'>): string {
  return item.href != null && item.href !== '' ? item.href : item.label
}

function pathMatches(pathname: string | null, href: string | undefined, mode: 'exact' | 'prefix') {
  if (pathname == null || href == null || href === '') return false
  return mode === 'exact' ? pathname === href : pathname.startsWith(href)
}

function shouldExpandSection(section: NavItem, pathname: string): boolean {
  if (pathMatches(pathname, section.href, 'prefix')) return true
  return section.items?.some(item => pathMatches(pathname, item.href, 'prefix')) === true
}

function shouldExpandItem(item: NavItem, pathname: string): boolean {
  if (pathMatches(pathname, item.href, 'prefix')) return true
  return item.items?.some(nested => pathMatches(pathname, nested.href, 'exact')) === true
}

function Chevron({ isOpen }: Readonly<{ isOpen: boolean }>) {
  return (
    <ChevronRight
      className={`w-4 h-4 transition-transform duration-200 ${isOpen ? 'rotate-90' : ''}`}
    />
  )
}

function ExpandButton({
  label,
  isOpen,
  onToggle,
  className = 'px-2 py-2.5',
}: Readonly<{
  label: string
  isOpen: boolean
  onToggle: () => void
  className?: string
}>) {
  return (
    <button
      type="button"
      onClick={onToggle}
      className={`${className} text-fg-muted hover:text-fg cursor-pointer`}
      aria-label={`${isOpen ? 'Collapse' : 'Expand'} ${label} section`}
      aria-expanded={isOpen}
    >
      <Chevron isOpen={isOpen} />
      <span className="sr-only">
        {isOpen ? 'Collapse' : 'Expand'} {label} section
      </span>
    </button>
  )
}

function NavLink({
  href,
  active,
  className,
  children,
  external,
  bullet,
}: Readonly<{
  href: string
  active: boolean
  className?: string
  children: ReactNode
  external?: boolean
  bullet?: boolean
}>) {
  return (
    <a
      href={href}
      {...(external === true ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
      className={`${linkBase} ${active ? linkActive : linkIdle} ${className ?? ''}`}
      aria-current={active ? 'page' : undefined}
    >
      {!active && <span className={hoverWash} />}
      <span className="relative z-10 flex items-center">
        {bullet === true && <span className="mr-2 text-fg-muted">•</span>}
        {children}
      </span>
    </a>
  )
}

function DocsTree({
  items,
  pathname,
  parentKey,
  expanded,
  toggle,
  depth,
}: Readonly<{
  items: readonly NavItem[]
  pathname: string | null
  parentKey: string
  expanded: Readonly<Record<string, boolean>>
  toggle: (key: string) => void
  depth: number
}>) {
  return (
    <ul className="mt-1 space-y-1">
      {items.map(item => {
        const key = depth === 0 ? navKey(item) : `${parentKey}-${navKey(item)}`
        const nested = item.items
        const hasNested = nested != null && nested.length > 0
        const isOpen = expanded[key] ?? true
        const showChevron = depth === 0 ? hasNested && item.collapsible === true : hasNested
        const pad = depth === 0 ? 'px-3 py-2 font-medium' : 'px-3 py-1.5'
        const active = pathMatches(pathname, item.href, 'exact')

        return (
          <li key={key}>
            <div className="flex items-center">
              {item.href != null && item.href !== '' ? (
                <NavLink
                  href={item.href}
                  active={active}
                  className={`${depth === 0 ? 'flex-1 block' : 'flex-1 flex items-center'} ${pad}`}
                  bullet={depth > 0 && !hasNested}
                >
                  {item.label}
                </NavLink>
              ) : (
                <div
                  className={
                    depth === 0
                      ? 'flex-1 px-3 py-2 text-xs text-fg-muted uppercase tracking-wider font-semibold'
                      : 'flex-1 flex items-center px-3 py-1.5 text-xs text-fg-muted font-medium'
                  }
                >
                  {item.label}
                </div>
              )}
              {showChevron && (
                <ExpandButton
                  label={item.label}
                  isOpen={isOpen}
                  onToggle={() => {
                    toggle(key)
                  }}
                  className={depth === 0 ? 'px-2 py-2' : 'px-2 py-1.5'}
                />
              )}
            </div>
            {hasNested && (showChevron ? isOpen : true) && (
              <DocsTree
                items={nested}
                pathname={pathname}
                parentKey={key}
                expanded={expanded}
                toggle={toggle}
                depth={depth + 1}
              />
            )}
          </li>
        )
      })}
    </ul>
  )
}

function useResetOnPathnameChange<T>(
  initialValue: T,
  pathname: string,
): [T, Dispatch<SetStateAction<T>>] {
  const [prevPathname, setPrevPathname] = useState(pathname)
  const [value, setValue] = useState(initialValue)

  if (pathname !== prevPathname) {
    setPrevPathname(pathname)
    setValue(initialValue)
    return [initialValue, setValue]
  }

  return [value, setValue]
}

export default function Sidebar({ version }: SidebarProps) {
  const pathname = usePathname()
  const isDocsPage = pathname.startsWith('/docs')
  const isEnterprisePage = pathname.startsWith('/enterprise')

  const [manualToggles, setManualToggles] = useResetOnPathnameChange<Record<string, boolean>>(
    {},
    pathname,
  )
  const [manualDocsToggle, setManualDocsToggle] = useResetOnPathnameChange<boolean | undefined>(
    undefined,
    pathname,
  )
  const [manualEnterpriseToggle, setManualEnterpriseToggle] = useResetOnPathnameChange<
    boolean | undefined
  >(undefined, pathname)

  const mobileToggleRef = useRef<HTMLInputElement>(null)
  const isDocsExpanded = manualDocsToggle ?? isDocsPage
  const isEnterpriseExpanded = manualEnterpriseToggle ?? isEnterprisePage

  const expandedSections = useMemo(() => {
    const sections: Record<string, boolean> = {}
    for (const section of docsNavigation) {
      const sectionKey = navKey(section)
      sections[sectionKey] = manualToggles[sectionKey] ?? shouldExpandSection(section, pathname)
      for (const item of section.items ?? []) {
        const itemKey = `${sectionKey}-${navKey(item)}`
        sections[itemKey] = manualToggles[itemKey] ?? shouldExpandItem(item, pathname)
      }
    }
    return sections
  }, [pathname, manualToggles])

  const toggleSection = (key: string) => {
    setManualToggles(prev => ({ ...prev, [key]: !expandedSections[key] }))
  }

  useEffect(() => {
    if (mobileToggleRef.current) mobileToggleRef.current.checked = false
  }, [pathname])

  return (
    <>
      <input
        type="checkbox"
        id="mobile-menu-toggle"
        className="peer hidden"
        ref={mobileToggleRef}
      />

      <label
        htmlFor="mobile-menu-toggle"
        className="peer-checked:fixed peer-checked:inset-0 peer-checked:bg-overlay peer-checked:z-20 hidden peer-checked:block lg:hidden"
      >
        <span className="sr-only">Close navigation menu</span>
      </label>

      <label
        htmlFor="mobile-menu-toggle"
        className="fixed top-4 left-4 z-50 lg:hidden bg-surface border border-edge rounded-md p-2 text-fg-muted hover:text-fg hover:bg-hover transition-colors duration-200 cursor-pointer peer-checked:hidden"
      >
        <Menu className="w-6 h-6" aria-hidden="true" />
        <span className="sr-only">Open navigation menu</span>
      </label>

      <nav className="fixed lg:relative -translate-x-full peer-checked:translate-x-0 lg:translate-x-0 transition-transform duration-300 ease-in-out z-40 h-screen lg:h-auto bg-canvas overflow-y-auto w-64 shrink-0">
        <label
          htmlFor="mobile-menu-toggle"
          className="absolute top-4 right-4 lg:hidden bg-surface border border-edge rounded-md p-2 text-fg-muted hover:text-fg hover:bg-hover transition-colors duration-200 cursor-pointer z-10"
        >
          <Close className="w-6 h-6" aria-hidden="true" />
          <span className="sr-only">Close navigation menu</span>
        </label>

        <div className="p-6">
          <div className="flex flex-row items-center lg:justify-between mb-8 pb-4 border-b border-edge/50 relative gap-3">
            <div className="absolute inset-x-0 bottom-0 h-px bg-linear-to-r from-transparent via-accent/30 to-transparent" />
            <a href="/" className="hover:opacity-80 transition-opacity" aria-label="rari home">
              <Rari className="w-14 h-8 text-fg" aria-hidden="true" />
            </a>
            <div className="px-2 py-1 bg-muted border border-accent/40 rounded-md text-xs text-fg font-mono font-medium w-fit">
              {version}
            </div>
          </div>

          <div className="mb-6">
            <SearchBar />
          </div>

          <ul className="space-y-1">
            {navigation.map(item => {
              const isDocs = item.id === 'docs'
              const isEnterprise = item.id === 'enterprise'
              const isSponsor = item.id === 'sponsor'
              const isActive = isDocs
                ? pathname === '/docs/getting-started'
                : isEnterprise
                  ? pathname === item.href
                  : pathMatches(pathname, item.href, 'prefix')
              const disabled = isDocs && pathname === '/docs/getting-started'

              return (
                <li key={item.id}>
                  <div className="flex items-center">
                    {disabled ? (
                      <div className="flex-1 block px-3 py-2.5 rounded-md text-sm font-medium text-fg-muted cursor-not-allowed">
                        {item.label}
                      </div>
                    ) : (
                      <NavLink
                        href={item.href}
                        active={isActive}
                        external={item.external}
                        className={`flex-1 px-3 py-2.5 font-medium ${isSponsor ? 'flex items-center' : 'block'} ${isActive ? 'border-l-2 border-accent' : ''}`}
                      >
                        {isSponsor && <Heart className="w-4 h-4 mr-2 text-pink-400" />}
                        {item.label}
                      </NavLink>
                    )}
                    {isDocs && (
                      <ExpandButton
                        label="documentation"
                        isOpen={isDocsExpanded}
                        onToggle={() => {
                          setManualDocsToggle(!isDocsExpanded)
                        }}
                      />
                    )}
                    {isEnterprise && (
                      <ExpandButton
                        label="enterprise"
                        isOpen={isEnterpriseExpanded}
                        onToggle={() => {
                          setManualEnterpriseToggle(!isEnterpriseExpanded)
                        }}
                      />
                    )}
                  </div>

                  {isEnterprise && isEnterpriseExpanded && item.items != null && (
                    <div className="mt-1">
                      <div className="space-y-1 ml-2 pl-3 border-l border-edge">
                        {item.items.map(sub => (
                          <NavLink
                            key={sub.href}
                            href={sub.href}
                            active={pathname === sub.href}
                            className="flex items-center px-3 py-1.5"
                            bullet
                          >
                            {sub.label}
                          </NavLink>
                        ))}
                      </div>
                    </div>
                  )}

                  {isDocs && isDocsExpanded && (
                    <div className="mt-1">
                      <div className="space-y-1 ml-2 pl-3 border-l border-edge">
                        <DocsTree
                          items={docsNavigation}
                          pathname={pathname}
                          parentKey=""
                          expanded={expandedSections}
                          toggle={toggleSection}
                          depth={0}
                        />
                      </div>
                    </div>
                  )}
                </li>
              )
            })}
          </ul>

          <div className="mt-8 pt-6 border-t border-edge/50 relative">
            <div className="absolute inset-x-0 top-0 h-px bg-linear-to-r from-transparent via-accent/30 to-transparent" />
            <ul className="space-y-3">
              <li className="flex justify-center">
                <ThemeSwitcher />
              </li>
              <li className="flex items-center justify-center gap-3">
                {(
                  [
                    [
                      'https://github.com/rari-build/rari',
                      'GitHub',
                      Github,
                      'from-accent/10 to-accent-hover/10',
                    ],
                    [
                      'https://discord.gg/GSh2Ak3b8Q',
                      'Discord',
                      Discord,
                      'from-indigo-500/10 to-purple-500/10',
                    ],
                    [
                      'https://bsky.app/profile/rari.build',
                      'Bluesky',
                      Bluesky,
                      'from-blue-500/10 to-cyan-500/10',
                    ],
                  ] as const
                ).map(([href, label, Icon, wash]) => (
                  <a
                    key={href}
                    href={href}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="p-2 text-fg-muted hover:text-fg hover:bg-hover rounded-md transition-all duration-200 relative overflow-hidden group"
                    aria-label={label}
                  >
                    <span
                      className={`absolute inset-0 bg-linear-to-r ${wash} opacity-0 group-hover:opacity-100 transition-opacity duration-300`}
                    />
                    <Icon className="w-5 h-5 relative z-10" />
                  </a>
                ))}
              </li>
            </ul>
          </div>
        </div>
      </nav>
    </>
  )
}
