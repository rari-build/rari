import type { DocumentHead } from '@qwik.dev/router'
import { component$ } from '@qwik.dev/core'
import { Form, routeAction$, routeLoader$, server$ } from '@qwik.dev/router'

// server$ defines a server-only function. From the client it round-trips via
// Qwik Router's RPC endpoint; called during SSR it runs directly. Qwik resolves
// the request event through node:async_hooks, which the rari runtime provides.
export const getServerStamp = server$(() => {
  return 'server-fn-ran'
})

// routeLoader$ runs on the server before render.
export const useGreeting = routeLoader$(async () => {
  // Await before calling server$ to force an async boundary: this only resolves
  // if the host's AsyncLocalStorage propagates the request event across awaits.
  await Promise.resolve()
  const stamp = await getServerStamp()
  return { message: 'loaded-on-the-server', stamp }
})

// routeAction$ runs on the server for the form POST (progressive enhancement
// without JS, client-side navigation data refresh with it).
export const useEcho = routeAction$(data => {
  return { echoed: `you said: ${String(data.name ?? '')}` }
})

export default component$(() => {
  const greeting = useGreeting()
  const echo = useEcho()
  return (
    <section>
      <h1>hello from qwik on rari</h1>
      <p>{greeting.value.message}</p>
      <p>{greeting.value.stamp}</p>
      <Form action={echo}>
        <input name="name" />
        <button type="submit">echo</button>
      </Form>
      {echo.value && <p>{echo.value.echoed}</p>}
    </section>
  )
})

export const head: DocumentHead = {
  title: 'Home — rari + qwik',
}
