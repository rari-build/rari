import type { ReactNode } from 'react'

export default function Template({ children }: { readonly children: ReactNode }) {
  return <div className="rari-page-shell">{children}</div>
}
