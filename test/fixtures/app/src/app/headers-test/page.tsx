import type { Metadata } from 'rari'
import { headers } from 'rari/headers'

export default async function HeadersTestPage() {
  const requestHeaders = await headers()
  const userAgent = requestHeaders.get('user-agent')
  const host = requestHeaders.get('host')

  return (
    <div>
      <h1>headers() Test</h1>
      <p data-testid="user-agent">{userAgent ?? 'missing'}</p>
      <p data-testid="host">{host ?? 'missing'}</p>
      <p data-testid="has-accept">{requestHeaders.has('accept') ? 'yes' : 'no'}</p>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'headers() Test',
}
