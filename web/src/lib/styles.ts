export const container = {
  base: 'max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full',
  marketing: 'max-w-5xl mx-auto px-4 sm:px-6 lg:px-8',
  section: 'w-full py-16 lg:py-24',
} as const

export const code = {
  panel: 'not-prose my-6 relative group overflow-hidden rounded-md border border-edge bg-surface max-w-full shadow-sm',
  header: 'flex items-center gap-2 bg-muted px-4 py-2.5 border-b border-edge',
  copyButton:
    'absolute right-2 p-1.5 text-fg-muted hover:text-fg bg-muted hover:bg-hover border border-edge rounded transition-all duration-200 opacity-100 lg:opacity-0 lg:group-hover:opacity-100 z-10',
} as const

export const text = {
  link: 'text-link hover:text-link-hover',
  accentGradient: 'text-transparent bg-clip-text bg-linear-to-r from-accent to-accent-hover',
} as const
