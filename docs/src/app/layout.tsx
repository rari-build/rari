/* eslint-disable react-refresh/only-export-components */
import type { LayoutProps } from 'rari/client'
import Sidebar from '../components/Sidebar'

interface NpmPackageInfo {
  'dist-tags': {
    latest: string
  }
}

async function fetchRariVersion(): Promise<string> {
  try {
    const response = await fetch('https://registry.npmjs.org/rari')
    if (!response.ok) {
      throw new Error(`Failed to fetch version: ${response.status}`)
    }

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
    <div className="min-h-screen bg-[#0d1117] text-gray-200 font-sans">
      <div className="flex min-h-screen">
        <Sidebar version={version} pathname={pathname} />
        <main className="flex-1 min-h-screen bg-[#0d1117]">
          <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8">
            {children}
          </div>
        </main>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Runtime Accelerated Rendering Infrastructure (Rari)',
  description:
    'Rari is a performance-first React framework powered by Rust. Build web applications with React Server Components, zero-config setup, and runtime-accelerated rendering infrastructure.',
}
