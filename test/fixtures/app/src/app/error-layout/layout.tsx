import type { ReactNode } from 'react'
import ErrorLayoutClient from './ErrorLayoutClient'

export default function ErrorLayoutTest({
  children,
}: {
  children: ReactNode
}) {
  return <ErrorLayoutClient>{children}</ErrorLayoutClient>
}
