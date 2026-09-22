import type { ReactNode } from 'react'
import { RootTemplateClient } from './template-client'

export default function RootTemplate({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <div className="rari-page-shell">
      <RootTemplateClient>{children}</RootTemplateClient>
    </div>
  )
}
