import { getRepoStars } from '@/lib/github-utils'
import Bluesky from './icons/Bluesky'
import Discord from './icons/Discord'
import Github from './icons/Github'

export default async function Footer() {
  const currentYear = new Date().getFullYear()
  const stars = await getRepoStars()

  return (
    <footer className="w-full bg-[#0d1117] rounded-t-md">
      <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-8 lg:py-4 lg:flex lg:items-center lg:justify-between lg:gap-x-3">
        <div className="flex items-center justify-center lg:justify-start lg:flex-1 gap-x-1.5 mt-3 lg:mt-0 lg:order-1">
          <p className="text-gray-300 text-sm">
            <a
              href="https://github.com/rari-build/rari/blob/main/LICENSE"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:underline hover:text-gray-200 transition-colors"
            >
              MIT License
            </a>
            {' '}
            ©
            {' '}
            {currentYear}
            {' '}
            Ryan Skinner
          </p>
        </div>

        <div className="lg:flex-1 flex items-center justify-center lg:justify-end gap-x-1.5 lg:order-3">
          <a
            href="https://github.com/rari-build/rari"
            target="_blank"
            rel="noopener noreferrer"
            className="rounded-md font-medium inline-flex items-center transition-all duration-200 px-2.5 py-1.5 text-sm gap-1.5 text-gray-300 hover:bg-[#21262d] hover:text-gray-100 relative overflow-hidden group"
            aria-label="rari on GitHub"
          >
            <span className="absolute inset-0 bg-linear-to-r from-[#fd7e14]/10 to-[#e8590c]/10 opacity-0 group-hover:opacity-100 transition-opacity duration-300"></span>
            <Github className="w-5 h-5 relative z-10" />
            {stars !== null && (
              <span className="text-xs text-gray-400 relative z-10">{stars.toLocaleString()}</span>
            )}
            <span className="sr-only">rari on GitHub</span>
          </a>

          <a
            href="https://discord.gg/GSh2Ak3b8Q"
            target="_blank"
            rel="noopener noreferrer"
            className="rounded-md font-medium inline-flex items-center transition-all duration-200 px-2.5 py-1.5 text-sm gap-1.5 text-gray-300 hover:bg-[#21262d] hover:text-gray-100 relative overflow-hidden group"
            aria-label="rari on Discord"
          >
            <span className="absolute inset-0 bg-linear-to-r from-indigo-500/10 to-purple-500/10 opacity-0 group-hover:opacity-100 transition-opacity duration-300"></span>
            <Discord className="w-5 h-5 relative z-10" />
            <span className="sr-only">rari on Discord</span>
          </a>

          <a
            href="https://bsky.app/profile/rari.build"
            target="_blank"
            rel="noopener noreferrer"
            className="rounded-md font-medium inline-flex items-center transition-all duration-200 px-2.5 py-1.5 text-sm gap-1.5 text-gray-300 hover:bg-[#21262d] hover:text-gray-100 relative overflow-hidden group"
            aria-label="rari on Bluesky"
          >
            <span className="absolute inset-0 bg-linear-to-r from-blue-500/10 to-cyan-500/10 opacity-0 group-hover:opacity-100 transition-opacity duration-300"></span>
            <Bluesky className="w-5 h-5 relative z-10" />
            <span className="sr-only">rari on Bluesky</span>
          </a>
        </div>
      </div>
    </footer>
  )
}
