import type { ReactNode } from 'react'
import { PageTransition } from './page-transition'

export default function Template({ children }: { readonly children: ReactNode }) {
  return <PageTransition>{children}</PageTransition>
}
