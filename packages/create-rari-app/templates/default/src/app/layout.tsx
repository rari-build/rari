import type { LayoutProps, Metadata } from 'rari'
import { SiteNav } from '@/components/SiteNav'
import './globals.css'

export default function Layout({ children }: LayoutProps) {
  return (
    <html lang="en">
      <head />
      <body className="min-h-screen bg-gray-50">
        <SiteNav />
        <main className="max-w-7xl mx-auto px-6 py-8">{children}</main>
      </body>
    </html>
  )
}

export const metadata: Metadata = {
  title: '{{PROJECT_NAME}}',
  description: 'A rari application',
}
