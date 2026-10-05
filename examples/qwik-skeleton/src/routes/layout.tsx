import { component$, Slot } from '@qwik.dev/core'

// Root layout (Qwik Router convention: src/routes/layout.tsx).
export default component$(() => {
  return (
    <div data-layout="root">
      <nav>rari + qwik</nav>
      <main>
        <Slot />
      </main>
    </div>
  )
})
