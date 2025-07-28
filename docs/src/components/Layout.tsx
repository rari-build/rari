import type { ReactNode } from 'react'
import { Link } from 'rari/client'
import { Suspense } from 'react'
import Version from './Version'

interface LayoutProps {
  children: ReactNode
  currentPage?: string
}

const navigation = [
  { href: '/', label: 'Home', id: 'home' },
  { href: '/getting-started', label: 'Getting Started', id: 'getting-started' },
]

export default function Layout({
  children,
  currentPage = 'home',
}: LayoutProps) {
  return (
    <div className="min-h-screen flex bg-[#0d1117] text-gray-200 font-sans">
      <nav className="w-64 bg-[#161b22] border-r border-[#30363d] fixed h-full overflow-y-auto">
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
                <Suspense fallback="..."><Version /></Suspense>
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
                  <svg
                    className="w-4 h-4 mr-2"
                    fill="currentColor"
                    viewBox="0 0 20 20"
                  >
                    <path
                      fillRule="evenodd"
                      d="M10 0C4.477 0 0 4.484 0 10.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0110 4.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.203 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.942.359.31.678.921.678 1.856 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0020 10.017C20 4.484 15.522 0 10 0z"
                      clipRule="evenodd"
                    />
                  </svg>
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
                  <svg
                    className="w-4 h-4 mr-2"
                    fill="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path d="M1.763 0C.786 0 0 .786 0 1.763v20.474C0 23.214.786 24 1.763 24h20.474c.977 0 1.763-.786 1.763-1.763V1.763C24 .786 23.214 0 22.237 0H1.763zM5.13 5.323l13.837.019-.009 5.183H13.82v9.42H5.113V5.323z" />
                  </svg>
                  npm
                </a>
              </li>
            </ul>
          </div>
        </div>
      </nav>

      <main className="flex-1 ml-64 min-h-screen bg-[#0d1117]">
        <div className="max-w-5xl mx-auto px-8 py-8">{children}</div>
      </main>
    </div>
  )
}
