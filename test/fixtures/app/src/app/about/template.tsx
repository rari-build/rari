import type { ReactNode } from 'react'
import { AboutTemplateClient } from './template-client'

export default function AboutTemplate({ children }: Readonly<{ children: ReactNode }>) {
  return <AboutTemplateClient>{children}</AboutTemplateClient>
}
