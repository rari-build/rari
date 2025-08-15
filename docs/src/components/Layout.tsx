import type { ReactNode } from 'react'
import { Link } from 'rari/client'
import { useEffect, useState } from 'react'
import { Bluesky } from './icons/Bluesky'
import { Github } from './icons/Github'
import { Npm } from './icons/Npm'
import Version from './Version'

interface LayoutProps {
  children: ReactNode
  currentPage?: string
  metaDescription?: string
}

const navigation = [
  { href: '/', label: 'Home', id: 'home' },
  { href: '/getting-started', label: 'Getting Started', id: 'getting-started' },
]

export default function Layout({
  children,
  currentPage = 'home',
  metaDescription,
}: LayoutProps) {
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false)

  useEffect(() => {
    if (metaDescription) {
      let metaTag = document.querySelector('meta[name="description"]')
      if (!metaTag) {
        metaTag = document.createElement('meta')
        metaTag.setAttribute('name', 'description')
        document.head.appendChild(metaTag)
      }
      metaTag.setAttribute('content', metaDescription)
    }
  }, [metaDescription])

  return (
    <div className="min-h-screen bg-[#0d1117] text-gray-200 font-sans">
      {isMobileMenuOpen && (
        <div
          className="fixed inset-0 bg-black bg-opacity-50 z-40 lg:hidden"
          style={{ willChange: 'opacity' }}
          onClick={() => setIsMobileMenuOpen(false)}
        />
      )}

      <button
        type="button"
        onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
        aria-label={
          isMobileMenuOpen ? 'Close navigation menu' : 'Open navigation menu'
        }
        className="fixed top-4 left-4 z-50 lg:hidden bg-[#161b22] border border-[#30363d] rounded-md p-2 text-gray-300 hover:text-white hover:bg-[#21262d] transition-colors duration-200"
      >
        <svg
          className="w-6 h-6"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M4 6h16M4 12h16M4 18h16"
          />
        </svg>
      </button>

      <div className="flex min-h-screen">
        <nav
          className={`fixed lg:relative lg:translate-x-0 transform transition-transform duration-300 ease-in-out z-40 h-screen lg:h-auto bg-[#161b22] border-r border-[#30363d] overflow-y-auto ${isMobileMenuOpen
            ? 'translate-x-0'
            : '-translate-x-full lg:translate-x-0'
          } w-64 flex-shrink-0`}
          style={{
            willChange: isMobileMenuOpen ? 'transform' : 'auto',
            contentVisibility: 'auto',
            containIntrinsicSize: '256px auto',
          }}
        >
          <div className="p-6">
            <div className="flex items-center space-x-3 mb-8 pb-4 border-b border-[#30363d]">
              <div className="w-8 h-8 bg-gradient-to-br from-[#fd7e14] to-[#e8590c] rounded-lg flex items-center justify-center shadow-lg">
                <span className="text-white font-bold text-lg">R</span>
              </div>
              <div>
                <span className="text-xl font-semibold text-[#f0f6fc] font-mono">
                  rari
                </span>
                <div className="text-xs text-gray-400 font-mono">
                  <Version />
                </div>
              </div>
            </div>

            <ul className="space-y-1">
              {navigation.map(item => (
                <li key={item.id}>
                  <Link
                    to={item.href}
                    className={`block px-3 py-2.5 rounded-md text-sm font-medium transition-all duration-200 ${currentPage === item.id
                      ? 'bg-[#1f2937] text-[#fd7e14] border-l-2 border-[#fd7e14] shadow-sm'
                      : 'text-gray-300 hover:bg-[#21262d] hover:text-gray-100'
                    }`}
                    style={{ willChange: 'background-color, color' }}
                  >
                    {item.label}
                  </Link>
                </li>
              ))}
            </ul>

            <div className="mt-8 pt-6 border-t border-[#30363d]">
              <p className="text-xs text-gray-500 uppercase tracking-wider font-semibold mb-3">
                Resources
              </p>
              <ul className="space-y-1">
                <li>
                  <a
                    href="https://github.com/rari-build/rari"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center px-3 py-2 text-sm text-gray-400 hover:text-gray-200 hover:bg-[#21262d] rounded-md transition-colors duration-200"
                  >
                    <Github className="w-4 h-4 mr-2" />
                    GitHub
                  </a>
                </li>
                <li>
                  <a
                    href="https://npmjs.com/package/rari"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center px-3 py-2 text-sm text-gray-400 hover:text-gray-200 hover:bg-[#21262d] rounded-md transition-colors duration-200"
                  >
                    <Npm className="w-4 h-4 mr-2" />
                    npm
                  </a>
                </li>
                <li>
                  <a
                    href="https://bsky.app/profile/rari.build"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center px-3 py-2 text-sm text-gray-400 hover:text-gray-200 hover:bg-[#21262d] rounded-md transition-colors duration-200"
                  >
                    <Bluesky className="w-4 h-4 mr-2" />
                    Bluesky
                  </a>
                </li>
              </ul>
            </div>
          </div>
        </nav>

        <main
          className="flex-1 min-h-screen bg-[#0d1117]"
          style={{ contentVisibility: 'auto' }}
        >
          <div
            className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8"
            style={{ containIntrinsicSize: '1280px auto' }}
          >
            {children}
          </div>
        </main>
      </div>
    </div>
  )
}
