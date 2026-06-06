import type { LayoutProps } from 'rari'

export default function AuthGroupLayout({ children }: LayoutProps) {
  return (
    <div>
      <div data-testid="auth-group-banner">Auth Group Banner</div>
      <div data-testid="auth-group-children">{children}</div>
    </div>
  )
}
