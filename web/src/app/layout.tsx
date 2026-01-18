import type { LayoutProps } from 'rari'
import Footer from '@/components/Footer'
import Sidebar from '@/components/Sidebar'

interface NpmPackageInfo {
  'dist-tags': {
    latest: string
  }
}

async function fetchRariVersion(): Promise<string> {
  try {
    const response = await fetch('https://registry.npmjs.org/rari')
    if (!response.ok)
      throw new Error(`Failed to fetch version: ${response.status}`)
    const data: NpmPackageInfo = await response.json()
    return data['dist-tags'].latest
  }
  catch (error) {
    console.error('Error fetching rari version:', error)
    return '0.0.0'
  }
}

export default async function RootLayout({ children, pathname }: LayoutProps) {
  const version = await fetchRariVersion()
  return (
    <div className="min-h-screen bg-[#30363d] text-gray-200 font-sans overflow-x-hidden" style={{ '--sidebar-width': 'calc(8rem)' } as React.CSSProperties}>
      <div className="flex min-h-screen">
        <Sidebar version={version} pathname={pathname} />
        <div className="flex-1 flex flex-col min-h-screen min-w-0 gap-0.5 pl-0.5 pr-0.5">
          <main className="flex-1 min-w-0 bg-[#0d1117] rounded-b-md overflow-hidden">
            {children}
          </main>
          <Footer />
        </div>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Runtime Accelerated Rendering Infrastructure (Rari)',
  description:
    'Rari is a performance-first React framework powered by Rust. Build web applications with React Server Components, zero-config setup, and runtime-accelerated rendering infrastructure.',
  icons: {
    icon: [
      { url: '/favicon.svg', type: 'image/svg+xml', sizes: 'any' },
      { url: '/favicon.ico', sizes: '32x32' },
    ],
    apple: [
      { url: '/apple-touch-icon.png', sizes: '180x180' },
    ],
    other: [
      { rel: 'mask-icon', url: '/safari-pinned-tab.svg', color: '#ffffff' },
    ],
  },
  themeColor: [
    { color: '#0d1117' },
    { media: '(prefers-color-scheme: dark)', color: '#0d1117' },
  ],
  appleWebApp: {
    title: 'Rari | Runtime Accelerated Rendering Infrastructure',
    statusBarStyle: 'black-translucent',
    capable: true,
  },
  openGraph: {
    title: 'Runtime Accelerated Rendering Infrastructure (Rari)',
    description: 'A performance-first React framework powered by Rust',
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
  },
}
