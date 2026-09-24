import type { LayoutProps, Metadata } from 'rari'
import { SiteNav } from './site-nav'
import './globals.css'

export default function Layout({ children }: LayoutProps) {
  return (
    <html lang="en">
      <head />
      <body className="min-h-screen">
        <div className="min-h-screen">
          <SiteNav />
          <main className="max-w-7xl mx-auto px-6 py-8">{children}</main>
        </div>
      </body>
    </html>
  )
}

export const metadata: Metadata = {
  title: 'rari Playground',
  description: 'Feature playground for the rari framework',
}
