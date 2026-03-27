'use client'

import type { Dispatch, SetStateAction } from 'react'
import type { NavItem } from '@/lib/docs-navigation'
import { usePathname } from 'rari/router'
import { useEffect, useMemo, useRef, useState } from 'react'
import { docsNavigation } from '@/lib/docs-navigation'
import Bluesky from './icons/Bluesky'
import ChevronRight from './icons/ChevronRight'
import Close from './icons/Close'
import Discord from './icons/Discord'
import Github from './icons/Github'
import Heart from './icons/Heart'
import Menu from './icons/Menu'
import Rari from './icons/Rari'
import SearchBar from './SearchBar'

interface SidebarProps {
  version: string
}

function shouldExpandSection(section: NavItem, pathname: string | null): boolean {
  if (section.href && pathname?.startsWith(section.href))
    return true

  if (section.items)
    return section.items.some(item => item.href && pathname?.startsWith(item.href))

  return false
}

function shouldExpandItem(item: NavItem, pathname: string | null): boolean {
  if (item.href && pathname?.startsWith(item.href))
    return true

  if (item.items)
    return item.items.some((nested: NavItem) => nested.href && pathname === nested.href)

  return false
}

const navigation = [
  { href: '/docs/getting-started', label: 'Docs', id: 'docs' },
  {
    href: '/enterprise',
    label: 'Enterprise',
    id: 'enterprise',
    items: [
      { href: '/enterprise/sponsors', label: 'Sponsors' },
    ],
  },
  { href: '/blog', label: 'Blog', id: 'blog' },
  { href: 'https://github.com/sponsors/skiniks', label: 'Become a Sponsor', id: 'sponsor', external: true },
]

function NavigationLink({
  item,
  isActive,
  isSponsor,
}: {
  item: typeof navigation[0]
  isActive: boolean
  isSponsor: boolean
}) {
  return (
    <a
      href={item.href}
      {...(isSponsor ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
      className={`flex-1 ${isSponsor ? 'flex items-center' : 'block'} px-3 py-2.5 rounded-md text-sm font-medium transition-all duration-200 relative overflow-hidden group ${isActive
        ? 'bg-linear-to-r from-[#fd7e14]/20 to-[#e8590c]/20 text-white border-l-2 border-[#fd7e14]'
        : 'text-gray-300 hover:bg-[#21262d] hover:text-gray-100'
      }`}
      aria-current={isActive ? 'page' : undefined}
    >
      {!isActive && (
        <span className={`absolute inset-0 ${isSponsor ? 'bg-linear-to-r from-pink-500/10 to-pink-600/10' : 'bg-linear-to-r from-[#fd7e14]/10 to-[#e8590c]/10'} opacity-0 group-hover:opacity-100 transition-opacity duration-300`}></span>
      )}
      {isSponsor && <Heart className="w-4 h-4 mr-2 text-pink-400 relative z-10" />}
      <span className="relative z-10">{item.label}</span>
    </a>
  )
}

function EnterpriseItems({
  item,
  pathname,
}: {
  item: typeof navigation[0]
  pathname: string | null
}) {
  if (!item.items || item.items.length === 0)
    return null

  return (
    <div className="mt-1">
      <div className="space-y-1 ml-2 pl-3 border-l border-[#30363d]">
        {item.items.map(subItem => (
          <a
            key={subItem.href}
            href={subItem.href}
            className={`flex items-center px-3 py-1.5 rounded-md text-sm transition-all duration-200 relative overflow-hidden group ${pathname === subItem.href
              ? 'bg-linear-to-r from-[#fd7e14]/20 to-[#e8590c]/20 text-white'
              : 'text-gray-300 hover:bg-[#21262d] hover:text-gray-100'
            }`}
          >
            {pathname !== subItem.href && (
              <span className="absolute inset-0 bg-linear-to-r from-[#fd7e14]/10 to-[#e8590c]/10 opacity-0 group-hover:opacity-100 transition-opacity duration-300"></span>
            )}
            <span className="relative z-10 flex items-center">
              <span className="mr-2 text-gray-400">•</span>
              {subItem.label}
            </span>
          </a>
        ))}
      </div>
    </div>
  )
}

function NestedDocItem({
  nestedItem,
  pathname,
}: {
  nestedItem: NavItem
  pathname: string | null
}) {
  return (
    <li>
      <a
        href={nestedItem.href}
        className={`flex items-center px-3 py-1.5 rounded-md text-sm transition-all duration-200 relative overflow-hidden group ${pathname === nestedItem.href
          ? 'bg-linear-to-r from-[#fd7e14]/20 to-[#e8590c]/20 text-white'
          : 'text-gray-300 hover:bg-[#21262d] hover:text-gray-100'
        }`}
      >
        {pathname !== nestedItem.href && (
          <span className="absolute inset-0 bg-linear-to-r from-[#fd7e14]/10 to-[#e8590c]/10 opacity-0 group-hover:opacity-100 transition-opacity duration-300"></span>
        )}
        <span className="relative z-10 flex items-center">
          <span className="mr-2 text-gray-400">•</span>
          {nestedItem.label}
        </span>
      </a>
    </li>
  )
}

function DocsSection({
  section,
  pathname,
  expandedSections,
  toggleSection,
}: {
  section: NavItem
  pathname: string | null
  expandedSections: Record<string, boolean>
  toggleSection: (key: string) => void
}) {
  const sectionKey = section.href || section.label
  const isSectionExpanded = expandedSections[sectionKey] ?? true
  const hasSectionItems = section.items && section.items.length > 0
  const showSectionChevron = hasSectionItems && section.collapsible === true

  return (
    <div>
      <div className="flex items-center">
        <div className="flex items-center">
          {section.href
            ? (
                <a
                  href={section.href}
                  className={`flex-1 block px-3 py-2 rounded-md text-sm font-medium transition-all duration-200 relative overflow-hidden group ${pathname === section.href
                    ? 'bg-linear-to-r from-[#fd7e14]/20 to-[#e8590c]/20 text-white'
                    : 'text-gray-300 hover:bg-[#21262d] hover:text-white'
                  }`}
                >
                  {pathname !== section.href && (
                    <span className="absolute inset-0 bg-linear-to-r from-[#fd7e14]/10 to-[#e8590c]/10 opacity-0 group-hover:opacity-100 transition-opacity duration-300"></span>
                  )}
                  <span className="relative z-10">{section.label}</span>
                </a>
              )
            : (
                <div className="flex-1 px-3 py-2 text-xs text-gray-400 uppercase tracking-wider font-semibold">
                  {section.label}
                </div>
              )}
          {showSectionChevron && (
            <button
              type="button"
              onClick={() => toggleSection(sectionKey)}
              className="px-2 py-2 text-gray-300 hover:text-gray-100 cursor-pointer"
              aria-label={isSectionExpanded ? `Collapse ${section.label} section` : `Expand ${section.label} section`}
              aria-expanded={isSectionExpanded}
            >
              <Chevron isOpen={isSectionExpanded} />
              <span className="sr-only">
                {isSectionExpanded ? 'Collapse' : 'Expand'}
                {' '}
                {section.label}
                {' '}
                section
              </span>
            </button>
          )}
        </div>
      </div>
      {hasSectionItems && (showSectionChevron ? isSectionExpanded : true) && section.items && (
        <ul className="mt-1 space-y-1">
          {section.items.map(subItem => (
            <DocsSectionItem
              key={subItem.href || subItem.label}
              subItem={subItem}
              sectionKey={sectionKey}
              pathname={pathname}
              expandedSections={expandedSections}
              toggleSection={toggleSection}
            />
          ))}
        </ul>
      )}
    </div>
  )
}

function DocsSectionItem({
  subItem,
  sectionKey,
  pathname,
  expandedSections,
  toggleSection,
}: {
  subItem: NavItem
  sectionKey: string
  pathname: string | null
  expandedSections: Record<string, boolean>
  toggleSection: (key: string) => void
}) {
  const itemKey = `${sectionKey}-${subItem.href || subItem.label}`
  const isItemExpanded = expandedSections[itemKey] ?? true
  const hasSubItems = subItem.items && subItem.items.length > 0
  const showItemChevron = hasSubItems

  return (
    <li>
      <div className="flex items-center">
        {subItem.href
          ? (
              <a
                href={subItem.href}
                className={`flex-1 flex items-center px-3 py-1.5 rounded-md text-sm transition-all duration-200 relative overflow-hidden group ${pathname === subItem.href
                  ? 'bg-linear-to-r from-[#fd7e14]/20 to-[#e8590c]/20 text-white'
                  : 'text-gray-300 hover:bg-[#21262d] hover:text-gray-100'
                }`}
              >
                {pathname !== subItem.href && (
                  <span className="absolute inset-0 bg-linear-to-r from-[#fd7e14]/10 to-[#e8590c]/10 opacity-0 group-hover:opacity-100 transition-opacity duration-300"></span>
                )}
                <span className="relative z-10 flex items-center">
                  {!hasSubItems && <span className="mr-2 text-gray-400">•</span>}
                  {subItem.label}
                </span>
              </a>
            )
          : (
              <div className="flex-1 flex items-center px-3 py-1.5 text-xs text-gray-400 font-medium">
                {subItem.label}
              </div>
            )}
        {showItemChevron && (
          <button
            type="button"
            onClick={() => toggleSection(itemKey)}
            className="px-2 py-1.5 text-gray-300 hover:text-gray-100 cursor-pointer"
            aria-label={isItemExpanded ? `Collapse ${subItem.label} section` : `Expand ${subItem.label} section`}
            aria-expanded={isItemExpanded}
          >
            <Chevron isOpen={isItemExpanded} />
            <span className="sr-only">
              {isItemExpanded ? 'Collapse' : 'Expand'}
              {' '}
              {subItem.label}
              {' '}
              section
            </span>
          </button>
        )}
      </div>
      {hasSubItems && (showItemChevron ? isItemExpanded : true) && subItem.items && (
        <ul className="mt-1 space-y-1">
          {subItem.items.map(nestedItem => (
            <NestedDocItem
              key={`${itemKey}-${nestedItem.href || nestedItem.label}`}
              nestedItem={nestedItem}
              pathname={pathname}
            />
          ))}
        </ul>
      )}
    </li>
  )
}

function DocsNavigation({
  pathname,
  expandedSections,
  toggleSection,
}: {
  pathname: string | null
  expandedSections: Record<string, boolean>
  toggleSection: (key: string) => void
}) {
  return (
    <div className="mt-1">
      <div className="space-y-1 ml-2 pl-3 border-l border-[#30363d]">
        {docsNavigation.map(section => (
          <DocsSection
            key={section.href || section.label}
            section={section}
            pathname={pathname}
            expandedSections={expandedSections}
            toggleSection={toggleSection}
          />
        ))}
      </div>
    </div>
  )
}

function NavigationItem({
  item,
  pathname,
  isDocsExpanded,
  isEnterpriseExpanded,
  expandedSections,
  setManualDocsToggle,
  setManualEnterpriseToggle,
  toggleSection,
}: {
  item: typeof navigation[0]
  pathname: string | null
  isDocsExpanded: boolean
  isEnterpriseExpanded: boolean
  expandedSections: Record<string, boolean>
  setManualDocsToggle: Dispatch<SetStateAction<boolean | undefined>>
  setManualEnterpriseToggle: Dispatch<SetStateAction<boolean | undefined>>
  toggleSection: (key: string) => void
}) {
  const isDocs = item.id === 'docs'
  const isEnterprise = item.id === 'enterprise'
  const isSponsor = item.id === 'sponsor'
  const isActive = isDocs
    ? pathname === '/docs/getting-started'
    : isEnterprise
      ? pathname === item.href
      : (pathname === item.href || pathname?.startsWith(item.href)) ?? false

  const isDisabled = isDocs && pathname === '/docs/getting-started'
  const hasItems = 'items' in item && item.items && item.items.length > 0

  return (
    <li>
      <div className="flex items-center">
        {isDisabled
          ? (
              <div className="flex-1 block px-3 py-2.5 rounded-md text-sm font-medium text-gray-400 cursor-not-allowed">
                {item.label}
              </div>
            )
          : (
              <NavigationLink item={item} isActive={isActive} isSponsor={isSponsor} />
            )}
        {isDocs && (
          <button
            type="button"
            onClick={() => setManualDocsToggle(!isDocsExpanded)}
            className="px-2 py-2.5 text-gray-300 hover:text-gray-100 cursor-pointer"
            aria-label={isDocsExpanded ? 'Collapse documentation section' : 'Expand documentation section'}
            aria-expanded={isDocsExpanded}
          >
            <Chevron isOpen={isDocsExpanded} />
            <span className="sr-only">
              {isDocsExpanded ? 'Collapse' : 'Expand'}
              {' '}
              documentation section
            </span>
          </button>
        )}
        {isEnterprise && (
          <button
            type="button"
            onClick={() => setManualEnterpriseToggle(!isEnterpriseExpanded)}
            className="px-2 py-2.5 text-gray-300 hover:text-gray-100 cursor-pointer"
            aria-label={isEnterpriseExpanded ? 'Collapse enterprise section' : 'Expand enterprise section'}
            aria-expanded={isEnterpriseExpanded}
          >
            <Chevron isOpen={isEnterpriseExpanded} />
            <span className="sr-only">
              {isEnterpriseExpanded ? 'Collapse' : 'Expand'}
              {' '}
              enterprise section
            </span>
          </button>
        )}
      </div>

      {isEnterprise && isEnterpriseExpanded && hasItems && (
        <EnterpriseItems item={item} pathname={pathname} />
      )}

      {isDocs && isDocsExpanded && (
        <DocsNavigation
          pathname={pathname}
          expandedSections={expandedSections}
          toggleSection={toggleSection}
        />
      )}
    </li>
  )
}

function Chevron({ isOpen }: { isOpen: boolean }) {
  return (
    <ChevronRight
      className={`w-4 h-4 transition-transform duration-200 ${isOpen ? 'rotate-90' : ''}`}
    />
  )
}

function useResetOnPathnameChange<T>(initialValue: T, pathname: string): [T, Dispatch<SetStateAction<T>>] {
  const lastPathnameRef = useRef(pathname)
  const [value, setValue] = useState(initialValue)

  if (pathname !== lastPathnameRef.current) {
    lastPathnameRef.current = pathname
    setValue(initialValue)
    return [initialValue, setValue]
  }

  return [value, setValue]
}

export default function Sidebar({ version }: SidebarProps) {
  const pathname = usePathname()
  const isDocsPage = pathname?.startsWith('/docs')
  const isEnterprisePage = pathname?.startsWith('/enterprise')

  const [manualToggles, setManualToggles] = useResetOnPathnameChange<Record<string, boolean>>({}, pathname)
  const [manualDocsToggle, setManualDocsToggle] = useResetOnPathnameChange<boolean | undefined>(undefined, pathname)
  const [manualEnterpriseToggle, setManualEnterpriseToggle] = useResetOnPathnameChange<boolean | undefined>(undefined, pathname)

  const mobileToggleRef = useRef<HTMLInputElement>(null)

  const isDocsExpanded = manualDocsToggle !== undefined ? manualDocsToggle : (isDocsPage ?? false)
  const isEnterpriseExpanded = manualEnterpriseToggle !== undefined ? manualEnterpriseToggle : (isEnterprisePage ?? false)

  const expandedSections = useMemo(() => {
    const sections: Record<string, boolean> = {}

    docsNavigation.forEach((section) => {
      const sectionKey = section.href || section.label
      sections[sectionKey] = manualToggles[sectionKey] ?? shouldExpandSection(section, pathname)

      if (section.items) {
        section.items.forEach((item) => {
          const itemKey = `${sectionKey}-${item.href || item.label}`
          sections[itemKey] = manualToggles[itemKey] ?? shouldExpandItem(item, pathname)
        })
      }
    })

    return sections
  }, [pathname, manualToggles])

  const toggleSection = (key: string) => {
    setManualToggles(prev => ({ ...prev, [key]: !expandedSections[key] }))
  }

  useEffect(() => {
    if (mobileToggleRef.current) {
      mobileToggleRef.current.checked = false
    }
  }, [pathname])

  return (
    <>
      <input type="checkbox" id="mobile-menu-toggle" className="peer hidden" ref={mobileToggleRef} />

      <label
        htmlFor="mobile-menu-toggle"
        className="peer-checked:fixed peer-checked:inset-0 peer-checked:bg-black/30 peer-checked:z-20 hidden peer-checked:block lg:hidden"
      />

      <label
        htmlFor="mobile-menu-toggle"
        className="fixed top-4 left-4 z-50 lg:hidden bg-[#161b22] border border-[#30363d] rounded-md p-2 text-gray-300 hover:text-white hover:bg-[#21262d] transition-colors duration-200 cursor-pointer peer-checked:hidden"
        aria-label="Open navigation menu"
      >
        <Menu className="w-6 h-6" />
        <span className="sr-only">Open navigation menu</span>
      </label>

      <nav className="fixed lg:relative -translate-x-full peer-checked:translate-x-0 lg:translate-x-0 transition-transform duration-300 ease-in-out z-40 h-screen lg:h-auto bg-[#0d1117] overflow-y-auto w-64 shrink-0">
        <label
          htmlFor="mobile-menu-toggle"
          className="absolute top-4 right-4 lg:hidden bg-[#161b22] border border-[#30363d] rounded-md p-2 text-gray-300 hover:text-white hover:bg-[#21262d] transition-colors duration-200 cursor-pointer z-10"
          aria-label="Close navigation menu"
        >
          <Close className="w-6 h-6" />
          <span className="sr-only">Close navigation menu</span>
        </label>

        <div className="p-6">
          <div className="flex flex-row items-center lg:justify-between mb-8 pb-4 border-b border-[#30363d]/50 relative gap-3">
            <div className="absolute inset-x-0 bottom-0 h-px bg-linear-to-r from-transparent via-[#fd7e14]/30 to-transparent"></div>
            <a
              href="/"
              className="hover:opacity-80 transition-opacity"
              aria-label="rari home"
            >
              <Rari className="w-14 h-8" aria-hidden="true" />
            </a>
            <div className="px-2 py-1 bg-[#161b22] border border-[#30363d] rounded-md text-xs text-[#fd7e14] font-mono font-medium w-fit">
              v
              {version}
            </div>
          </div>

          <div className="mb-6">
            <SearchBar />
          </div>

          <ul className="space-y-1">
            {navigation.map(item => (
              <NavigationItem
                key={item.id}
                item={item}
                pathname={pathname}
                isDocsExpanded={isDocsExpanded}
                isEnterpriseExpanded={isEnterpriseExpanded}
                expandedSections={expandedSections}
                setManualDocsToggle={setManualDocsToggle}
                setManualEnterpriseToggle={setManualEnterpriseToggle}
                toggleSection={toggleSection}
              />
            ))}
          </ul>

          <div className="mt-8 pt-6 border-t border-[#30363d]/50 relative">
            <div className="absolute inset-x-0 top-0 h-px bg-linear-to-r from-transparent via-[#fd7e14]/30 to-transparent"></div>
            <ul className="space-y-3">
              <li className="flex items-center justify-center gap-3">
                <a
                  href="https://github.com/rari-build/rari"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="p-2 text-gray-300 hover:text-gray-100 hover:bg-[#21262d] rounded-md transition-all duration-200 relative overflow-hidden group"
                  aria-label="GitHub"
                >
                  <span className="absolute inset-0 bg-linear-to-r from-[#fd7e14]/10 to-[#e8590c]/10 opacity-0 group-hover:opacity-100 transition-opacity duration-300"></span>
                  <Github className="w-5 h-5 relative z-10" />
                </a>
                <a
                  href="https://discord.gg/GSh2Ak3b8Q"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="p-2 text-gray-300 hover:text-gray-100 hover:bg-[#21262d] rounded-md transition-all duration-200 relative overflow-hidden group"
                  aria-label="Discord"
                >
                  <span className="absolute inset-0 bg-linear-to-r from-indigo-500/10 to-purple-500/10 opacity-0 group-hover:opacity-100 transition-opacity duration-300"></span>
                  <Discord className="w-5 h-5 relative z-10" />
                </a>
                <a
                  href="https://bsky.app/profile/rari.build"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="p-2 text-gray-300 hover:text-gray-100 hover:bg-[#21262d] rounded-md transition-all duration-200 relative overflow-hidden group"
                  aria-label="Bluesky"
                >
                  <span className="absolute inset-0 bg-linear-to-r from-blue-500/10 to-cyan-500/10 opacity-0 group-hover:opacity-100 transition-opacity duration-300"></span>
                  <Bluesky className="w-5 h-5 relative z-10" />
                </a>
              </li>
            </ul>
          </div>
        </div>
      </nav>
    </>
  )
}
