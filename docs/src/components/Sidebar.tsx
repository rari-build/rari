import Bluesky from './icons/Bluesky'
import Close from './icons/Close'
import Discord from './icons/Discord'
import Github from './icons/Github'
import Heart from './icons/Heart'
import Menu from './icons/Menu'

interface SidebarProps {
  version: string
  pathname?: string
}

const navigation = [
  { href: '/', label: 'Home', id: 'home' },
  { href: '/getting-started', label: 'Getting Started', id: 'getting-started' },
]

export default function Sidebar({ version, pathname = '/' }: SidebarProps) {
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
      >
        <Menu className="w-6 h-6" />
      </label>

      <nav className="fixed lg:relative -translate-x-full peer-checked:translate-x-0 lg:translate-x-0 transition-transform duration-300 ease-in-out z-40 h-screen lg:h-auto bg-[#161b22] border-r border-[#30363d] overflow-y-auto w-64 shrink-0">
        <label
          htmlFor="mobile-menu-toggle"
          className="absolute top-4 right-4 lg:hidden bg-[#161b22] border border-[#30363d] rounded-md p-2 text-gray-300 hover:text-white hover:bg-[#21262d] transition-colors duration-200 cursor-pointer z-10"
        >
          <Close className="w-6 h-6" />
        </label>

        <div className="p-6">
          <div className="flex items-center space-x-3 mb-8 pb-4 border-b border-[#30363d]">
            <div className="w-8 h-8 bg-linear-to-br from-[#fd7e14] to-[#e8590c] rounded-lg flex items-center justify-center shadow-lg">
              <span className="text-white font-bold text-lg">R</span>
            </div>
            <div>
              <span className="text-xl font-semibold text-[#f0f6fc] font-mono">
                rari
              </span>
              <div className="text-xs text-gray-400 font-mono">
                v
                {version}
              </div>
            </div>
          </div>

          <ul className="space-y-1">
            {navigation.map((item) => {
              const isActive = pathname === item.href
              return (
                <li key={item.id}>
                  <a
                    href={item.href}
                    className={`block px-3 py-2.5 rounded-md text-sm font-medium transition-all duration-200 ${isActive
                      ? 'bg-[#21262d] text-white'
                      : 'text-gray-300 hover:bg-[#21262d] hover:text-gray-100'
                    }`}
                    aria-current={isActive ? 'page' : undefined}
                  >
                    {item.label}
                  </a>
                </li>
              )
            })}
          </ul>

          <div className="mt-8 pt-6 border-t border-[#30363d]">
            <p className="text-xs text-gray-500 uppercase tracking-wider font-semibold mb-3">
              Resources
            </p>
            <ul className="space-y-1">
              <li>
                <a
                  href="https://github.com/sponsors/skiniks"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center px-3 py-2 text-sm text-gray-400 hover:text-gray-200 hover:bg-[#21262d] rounded-md transition-colors duration-200"
                >
                  <Heart className="w-4 h-4 mr-2 text-red-600" />
                  Become a Sponsor
                </a>
              </li>
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
                  href="https://discord.gg/GSh2Ak3b8Q"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center px-3 py-2 text-sm text-gray-400 hover:text-gray-200 hover:bg-[#21262d] rounded-md transition-colors duration-200"
                >
                  <Discord className="w-4 h-4 mr-2" />
                  Discord
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
    </>
  )
}
