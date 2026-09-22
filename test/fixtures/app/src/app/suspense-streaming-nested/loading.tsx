import { LoadingReveal } from '../page-transition'

export default function SuspenseStreamingNestedLoading() {
  return (
    <LoadingReveal>
      <div data-testid="page-loading">Loading...</div>
    </LoadingReveal>
  )
}
