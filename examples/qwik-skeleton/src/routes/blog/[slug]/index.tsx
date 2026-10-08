import type { DocumentHead } from '@qwik.dev/router'
import type { RariPlatform } from '@rari/qwik'
import { component$ } from '@qwik.dev/core'
import { routeLoader$, useLocation } from '@qwik.dev/router'

// Dynamic route: src/routes/blog/[slug]/index.tsx -> /blog/:slug.
//
// rari matched this URL before Qwik ran; the match is on `platform.rari.route`
// and is the host's decision (pattern, params, layout chain).
export const useHostRoute = routeLoader$(({ platform }) => {
  const route = (platform as RariPlatform).rari.route
  return route ? `${route.path} ${String(route.params.slug)}` : 'no host route'
})

export default component$(() => {
  const location = useLocation()
  const hostRoute = useHostRoute()
  return (
    <section>
      <h1>blog post: {location.params.slug}</h1>
      <p data-testid="host-route">{hostRoute.value}</p>
    </section>
  )
})

export const head: DocumentHead = ({ params }) => ({
  title: `Blog: ${params.slug}`,
})
