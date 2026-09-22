import { LoadingReveal } from '../page-transition'

export default function SuspenseStreamingLoading() {
  return (
    <LoadingReveal>
      <div data-testid="page-loading">Loading...</div>
    </LoadingReveal>
  )
}
