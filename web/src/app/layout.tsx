import type { LayoutProps, Metadata } from 'rari'
import type { CSSProperties } from 'react'
import Footer from '@/components/Footer'
import { Providers } from '@/components/Providers'
import Sidebar from '@/components/Sidebar'
import { getLatestRariVersion } from '@/lib/github'
import { siteUrl } from '@/lib/site'
import './globals.css'

export default async function Layout({ children, pathname }: LayoutProps) {
  const version = await getLatestRariVersion()

  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <script
          // eslint-disable-next-line react/dom-no-dangerously-set-innerhtml
          dangerouslySetInnerHTML={{
            __html: `(function(){try{var stored=localStorage.getItem('preferred-theme');var preference=stored==='light'||stored==='dark'||stored==='system'?stored:'system';var resolved=preference==='system'?window.matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light':preference;document.documentElement.classList.toggle('light',resolved==='light');document.documentElement.classList.toggle('dark',resolved==='dark')}catch(e){document.documentElement.classList.add('dark')}})()`,
          }}
        />
      </head>
      <body className="min-h-screen bg-canvas text-fg-body">
        <Providers pathname={pathname}>
          <div
            className="min-h-screen bg-chrome text-fg-body font-sans overflow-x-hidden"
            // oxlint-disable-next-line typescript/no-unsafe-type-assertion CSS custom property
            style={{ '--sidebar-width': 'calc(8rem)' } as CSSProperties}
          >
            <div className="flex min-h-screen">
              <Sidebar version={version} />
              <div className="flex-1 flex flex-col min-h-screen min-w-0 gap-0.5 md:pl-0.5 md:pr-0.5">
                <main className="flex-1 min-w-0 bg-canvas rounded-b-md overflow-hidden">
                  {children}
                </main>
                <Footer />
              </div>
            </div>
          </div>
        </Providers>
      </body>
    </html>
  )
}

export const metadata: Metadata = {
  title: 'Runtime Accelerated Rendering Infrastructure (rari)',
  description:
    'rari is a performance-first React framework powered by Rust. Build web applications with React Server Components, zero-config setup, and runtime-accelerated rendering infrastructure.',
  themeColor: [
    { media: '(prefers-color-scheme: light)', color: '#e8ecf1' },
    { media: '(prefers-color-scheme: dark)', color: '#0d1117' },
  ],
  appleWebApp: {
    title: 'rari | Runtime Accelerated Rendering Infrastructure',
    statusBarStyle: 'default',
    capable: true,
  },
  openGraph: {
    title: 'Runtime Accelerated Rendering Infrastructure (rari)',
    description: 'A performance-first React framework powered by Rust',
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
  },
  alternates: {
    types: {
      'application/rss+xml': `${siteUrl}/feed.xml`,
    },
  },
}
