import type { DocumentHead } from '@qwik.dev/router'
import { component$ } from '@qwik.dev/core'
import { useLocation } from '@qwik.dev/router'

// Dynamic route: src/routes/blog/[slug]/index.tsx → /blog/:slug.
export default component$(() => {
  const location = useLocation()
  return <h1>blog post: {location.params.slug}</h1>
})

export const head: DocumentHead = ({ params }) => ({
  title: `Blog: ${params.slug}`,
})
