import type { DocumentHead } from '@qwik.dev/router'
import { component$, Pending } from '@qwik.dev/core'

const SlowContent = component$(() => {
  const content = new Promise<string>(resolve => {
    setTimeout(() => resolve('streamed-in-out-of-order'), 300)
  })
  return <p>{content}</p>
})

// Out-of-order streaming: the shell and fallback flush immediately, the slow
// content streams in ~300ms later and the client swaps it into place.
export default component$(() => {
  return (
    <section>
      <h1>out-of-order streaming</h1>
      <Pending fallback$={() => <p>loading-fallback</p>}>
        <SlowContent />
      </Pending>
      <footer>shell rendered immediately</footer>
    </section>
  )
})

export const head: DocumentHead = { title: 'Slow — rari + qwik' }
