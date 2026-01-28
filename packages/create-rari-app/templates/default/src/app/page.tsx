import type { PageProps, Metadata } from 'rari'
import ServerTime from '@/components/ServerTime'
import Welcome from '@/components/Welcome'

export default function HomePage(_params: PageProps) {
  return (
    <div className="space-y-8">
      <Welcome />
      <ServerTime />
    </div>
  )
}

export const metadata: Metadata = {
  title: 'Home | {{PROJECT_NAME}}',
  description: 'Welcome to your new rari application',
}
