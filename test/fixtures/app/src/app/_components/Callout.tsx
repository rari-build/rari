import type { ReactNode } from 'react'

export function Callout({ children }: { children: ReactNode }) {
  return (
    <aside data-testid="private-callout" className="callout">
      {children}
    </aside>
  )
}
