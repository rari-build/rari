import type { ReactNode } from 'react'
import { PageTransition } from './page-transition'
import { RootTemplateClient } from './template-client'

export default function RootTemplate({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <PageTransition>
      <RootTemplateClient>{children}</RootTemplateClient>
    </PageTransition>
  )
}
