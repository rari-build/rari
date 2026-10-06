import type { RariPlatform } from '@rari/qwik'
import { component$ } from '@qwik.dev/core'
import { routeLoader$ } from '@qwik.dev/router'

// `index@narrow` selects the `narrow` layout variant: rari's route-file
// grammar, resolved by the host. The chain it resolved is on the route decision.
export const useHostLayouts = routeLoader$(({ platform }) => {
  const route = (platform as RariPlatform).rari.route
  return route ? route.layouts.join(',') : 'no host route'
})

export default component$(() => {
  const layouts = useHostLayouts()
  return (
    <>
      <h1>narrow page via named layout</h1>
      <p data-testid="host-layouts">{layouts.value}</p>
    </>
  )
})
