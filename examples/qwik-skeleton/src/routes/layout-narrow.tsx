import { component$, Slot } from '@qwik.dev/core'

// Named layout (Qwik convention `layout-<name>.tsx`), opted into by
// `index@narrow.tsx` pages. rari's scanner must still see those pages.
export default component$(() => {
  return (
    <div data-layout="narrow" style="max-width: 40rem">
      <Slot />
    </div>
  )
})
