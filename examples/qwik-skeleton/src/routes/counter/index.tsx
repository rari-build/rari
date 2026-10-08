import { component$, useSignal } from '@qwik.dev/core'

// Pure client reactivity: SSR renders count: 0; clicking must resume the handler
// and increment. Proves resumable SSR + the served segment bundles work in the
// browser, not just that the HTML looks right.
export default component$(() => {
  const count = useSignal(0)
  return (
    <section>
      <h1>counter</h1>
      <button type="button" data-testid="btn" onClick$={() => count.value++}>
        count: {count.value}
      </button>
    </section>
  )
})
