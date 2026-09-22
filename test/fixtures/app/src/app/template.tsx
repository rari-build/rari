import type { ReactNode } from 'react'
import { RootTemplateClient } from './template-client'

export default function RootTemplate({ children }: Readonly<{ children: ReactNode }>) {
  return <RootTemplateClient>{children}</RootTemplateClient>
}
