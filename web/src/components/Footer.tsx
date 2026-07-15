import { formatCompactNumber } from '@/lib/format'
import { getLatestCommitHash, getRepoStars } from '@/lib/github'
import { container } from '@/lib/styles'
import Bluesky from './icons/Bluesky'
import Discord from './icons/Discord'
import Github from './icons/Github'

export default async function Footer() {
  // eslint-disable-next-line react/purity
  const currentYear = new Date().getFullYear()
  const stars = await getRepoStars()
  const commitHash = await getLatestCommitHash()

  return (
    <footer className="w-full bg-canvas rounded-t-md">
      <div className={`${container.marketing} py-8 lg:py-4 lg:flex lg:items-center lg:justify-between lg:gap-x-3`}>
        <div className="flex items-center justify-center lg:justify-start lg:flex-1 gap-x-1.5 mt-3 lg:mt-0 lg:order-1">
          <p className="text-fg-muted text-sm">
            <a
              href="https://github.com/rari-build/rari/blob/main/LICENSE"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:underline hover:text-fg-secondary transition-colors"
            >
              MIT License
            </a>
            {' '}
            ©
            {' '}
            {currentYear}
            {' '}
            Ryan Skinner
            {commitHash && (
              <>
                {' '}
                (
                <a
                  href={`https://github.com/rari-build/rari/commit/${commitHash}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:underline hover:text-fg-secondary transition-colors"
                >
                  {commitHash}
                </a>
                )
              </>
            )}
          </p>
        </div>

        <div className="lg:flex-1 flex items-center justify-center lg:justify-end gap-x-1.5 lg:order-3">
          <a
            href="https://github.com/rari-build/rari"
            target="_blank"
            rel="noopener noreferrer"
            className="rounded-md font-medium inline-flex items-center transition-all duration-200 px-2.5 py-1.5 text-sm gap-1.5 text-fg-muted hover:bg-hover hover:text-fg relative overflow-hidden group"
            aria-label="rari on GitHub"
          >
            <span className="absolute inset-0 bg-linear-to-r from-accent/10 to-accent-hover/10 opacity-0 group-hover:opacity-100 transition-opacity duration-300"></span>
            <Github className="w-5 h-5 relative z-10" />
            {stars !== null && (
              <span className="text-xs text-fg-muted relative z-10">{formatCompactNumber(stars)}</span>
            )}
            <span className="sr-only">rari on GitHub</span>
          </a>

          <a
            href="https://discord.gg/GSh2Ak3b8Q"
            target="_blank"
            rel="noopener noreferrer"
            className="rounded-md font-medium inline-flex items-center transition-all duration-200 px-2.5 py-1.5 text-sm gap-1.5 text-fg-muted hover:bg-hover hover:text-fg relative overflow-hidden group"
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
            className="rounded-md font-medium inline-flex items-center transition-all duration-200 px-2.5 py-1.5 text-sm gap-1.5 text-fg-muted hover:bg-hover hover:text-fg relative overflow-hidden group"
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
