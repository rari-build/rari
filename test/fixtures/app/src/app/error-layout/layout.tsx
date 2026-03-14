import ErrorLayoutClient from './ErrorLayoutClient'

export default function ErrorLayoutTest({
  children,
}: {
  children: React.ReactNode
}) {
  return <ErrorLayoutClient>{children}</ErrorLayoutClient>
}
