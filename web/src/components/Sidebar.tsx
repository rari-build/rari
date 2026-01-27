'use client'

import { useMemo, useState } from 'react'
import { docsNavigation } from '@/lib/navigation'
import Bluesky from './icons/Bluesky'
import ChevronRight from './icons/ChevronRight'
import Close from './icons/Close'
import Discord from './icons/Discord'
import Github from './icons/Github'
import Heart from './icons/Heart'
import Menu from './icons/Menu'
import Rari from './icons/Rari'

interface SidebarProps {
  version: string
  pathname?: string
}

const navigation = [
  { href: '/docs', label: 'Docs', id: 'docs' },
  { href: '/blog', label: 'Blog', id: 'blog' },
  { href: 'https://github.com/sponsors/skiniks', label: 'Become a Sponsor', id: 'sponsor', external: true },
]

function Chevron({ isOpen }: { isOpen: boolean }) {
  return (
    <ChevronRight
      className={`w-4 h-4 transition-transform duration-200 ${isOpen ? 'rotate-90' : ''}`}
    />
  )
}

export default function Sidebar({ version, pathname = '/' }: SidebarProps) {
  const isDocsPage = pathname?.startsWith('/docs')
  const [manualToggles, setManualToggles] = useState<Record<string, boolean>>({})
  const [manualDocsToggle, setManualDocsToggle] = useState<boolean | undefined>(undefined)
  const [lastPathname, setLastPathname] = useState(pathname)

  const currentManualToggles = pathname !== lastPathname ? {} : manualToggles
  const currentManualDocsToggle = pathname !== lastPathname ? undefined : manualDocsToggle

  if (pathname !== lastPathname) {
    setLastPathname(pathname)
    if (Object.keys(manualToggles).length > 0) {
      setManualToggles({})
    }
    if (manualDocsToggle !== undefined) {
      setManualDocsToggle(undefined)
    }
  }

  const isDocsExpanded = currentManualDocsToggle !== undefined ? currentManualDocsToggle : isDocsPage

  const expandedSections = useMemo(() => {
    const sections: Record<string, boolean> = {}

    docsNavigation.forEach((section, idx) => {
      const sectionKey = `section-${idx}`

      if (currentManualToggles[sectionKey] !== undefined) {
        sections[sectionKey] = currentManualToggles[sectionKey]
      }
      else {
        let shouldExpand = true
        if (section.href && pathname?.startsWith(section.href)) {
          shouldExpand = true
        }
        else if (section.items) {
          const hasActiveChild = section.items.some(item =>
            item.href && pathname?.startsWith(item.href),
          )
          if (hasActiveChild) {
            shouldExpand = true
          }
        }
        sections[sectionKey] = shouldExpand
      }

      if (section.items) {
        section.items.forEach((item, itemIdx) => {
          const itemKey = `${sectionKey}-item-${itemIdx}`

          if (currentManualToggles[itemKey] !== undefined) {
            sections[itemKey] = currentManualToggles[itemKey]
          }
          else {
            let shouldExpand = true
            if (item.href && pathname?.startsWith(item.href)) {
              shouldExpand = true
            }
            else if (item.items) {
              const hasActiveNestedChild = item.items.some(nested =>
                nested.href && pathname === nested.href,
              )
              if (hasActiveNestedChild) {
                shouldExpand = true
              }
            }
            sections[itemKey] = shouldExpand
          }
        })
      }
    })

    return sections
  }, [pathname, currentManualToggles])

  const toggleSection = (key: string) => {
    setManualToggles(prev => ({ ...prev, [key]: !expandedSections[key] }))
  }

  return (
    <>
      <input type="checkbox" id="mobile-menu-toggle" className="peer hidden" />

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
          <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between mb-8 pb-4 border-b border-[#30363d]/50 relative gap-3">
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

          <ul className="space-y-1">
            {navigation.map((item) => {
              const isDocs = item.id === 'docs'
              const isSponsor = item.id === 'sponsor'
              const isActive = isDocs
                ? pathname === '/docs'
                : item.href === '/'
                  ? pathname === item.href
                  : pathname?.startsWith(item.href)

              const isDisabled = isDocs && pathname === '/docs/getting-started'

              return (
                <li key={item.id}>
                  <div className="flex items-center">
                    {isDisabled
                      ? (
                          <div className="flex-1 block px-3 py-2.5 rounded-md text-sm font-medium text-gray-500 cursor-not-allowed">
                            {item.label}
                          </div>
                        )
                      : (
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
                  </div>

                  {isDocs && isDocsExpanded && (
                    <div className="mt-1">
                      <div className="space-y-1 ml-2 pl-3 border-l border-[#30363d]">
                        {docsNavigation.map((section, idx) => {
                          const sectionKey = `section-${idx}`
                          const isSectionExpanded = expandedSections[sectionKey] !== undefined ? expandedSections[sectionKey] : true
                          const hasSectionItems = section.items && section.items.length > 0
                          const showSectionChevron = hasSectionItems && section.label === 'Getting Started'

                          return (
                            <div key={idx}>
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
                              {hasSectionItems && (showSectionChevron ? isSectionExpanded : true) && (
                                <ul className="mt-1 space-y-1">
                                  {section.items!.map((subItem, itemIdx) => {
                                    const itemKey = `${sectionKey}-item-${itemIdx}`
                                    const isItemExpanded = expandedSections[itemKey] !== undefined ? expandedSections[itemKey] : true
                                    const hasSubItems = subItem.items && subItem.items.length > 0
                                    const showItemChevron = hasSubItems && subItem.label === 'Components'

                                    return (
                                      <li key={itemIdx}>
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
                                                    {!hasSubItems && <span className="mr-2 text-gray-500">•</span>}
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
                                        {hasSubItems && (showItemChevron ? isItemExpanded : true) && (
                                          <ul className="mt-1 space-y-1">
                                            {subItem.items!.map((nestedItem, subIdx) => (
                                              <li key={subIdx}>
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
                                                    <span className="mr-2 text-gray-500">•</span>
                                                    {nestedItem.label}
                                                  </span>
                                                </a>
                                              </li>
                                            ))}
                                          </ul>
                                        )}
                                      </li>
                                    )
                                  })}
                                </ul>
                              )}
                            </div>
                          )
                        })}
                      </div>
                    </div>
                  )}
                </li>
              )
            })}
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
