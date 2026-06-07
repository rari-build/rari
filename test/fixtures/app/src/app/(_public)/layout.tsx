import type { LayoutProps } from 'rari'

export default function PublicGroupLayout({ children }: LayoutProps) {
  return (
    <div>
      <div data-testid="public-group-banner">Public Group Banner</div>
      <div data-testid="public-group-children">{children}</div>
    </div>
  )
}
