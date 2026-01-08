import Bluesky from './icons/Bluesky'
import Discord from './icons/Discord'
import Github from './icons/Github'

interface GitHubRepo {
  stargazers_count: number
}

async function fetchGitHubStars(): Promise<number | null> {
  try {
    const response = await fetch('https://api.github.com/repos/rari-build/rari')
    if (!response.ok)
      return null
    const data: GitHubRepo = await response.json()
    return data.stargazers_count
  }
  catch (error) {
    console.error('Error fetching GitHub stars:', error)
    return null
  }
}

export default async function Footer() {
  const currentYear = new Date().getFullYear()
  const stars = await fetchGitHubStars()

  return (
    <footer className="w-full border-t border-[#30363d] bg-[#0d1117] mt-auto">
      <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-8 lg:py-4 lg:flex lg:items-center lg:justify-between lg:gap-x-3">
        <div className="flex items-center justify-center lg:justify-start lg:flex-1 gap-x-1.5 mt-3 lg:mt-0 lg:order-1">
          <p className="text-gray-400 text-sm">
            <a
              href="https://github.com/rari-build/rari/blob/main/LICENSE"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:underline hover:text-gray-300 transition-colors"
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
            className="rounded-md font-medium inline-flex items-center transition-colors px-2.5 py-1.5 text-sm gap-1.5 text-gray-400 hover:bg-[#21262d] hover:text-gray-200"
            aria-label="Rari on GitHub"
          >
            <Github className="w-5 h-5" />
            {stars !== null && (
              <span className="text-xs text-gray-500">{stars.toLocaleString()}</span>
            )}
            <span className="sr-only">Rari on GitHub</span>
          </a>

          <a
            href="https://discord.gg/GSh2Ak3b8Q"
            target="_blank"
            rel="noopener noreferrer"
            className="rounded-md font-medium inline-flex items-center transition-colors px-2.5 py-1.5 text-sm gap-1.5 text-gray-400 hover:bg-[#21262d] hover:text-gray-200"
            aria-label="Rari on Discord"
          >
            <Discord className="w-5 h-5" />
            <span className="sr-only">Rari on Discord</span>
          </a>

          <a
            href="https://bsky.app/profile/rari.build"
            target="_blank"
            rel="noopener noreferrer"
            className="rounded-md font-medium inline-flex items-center transition-colors px-2.5 py-1.5 text-sm gap-1.5 text-gray-400 hover:bg-[#21262d] hover:text-gray-200"
            aria-label="Rari on Bluesky"
          >
            <Bluesky className="w-5 h-5" />
            <span className="sr-only">Rari on Bluesky</span>
          </a>
        </div>
      </div>
    </footer>
  )
}
