import { component$, useSignal } from '@qwik.dev/core'

export default component$(() => {
  const count = useSignal(0)

  return (
    <div class="p-6 border border-gray-300 rounded-lg bg-white">
      <h3 class="text-lg font-semibold mb-4">Counter Component</h3>
      <p class="text-gray-600 mb-4">This is a client component with state and interactivity.</p>

      <div class="flex items-center gap-4 mb-4">
        <button
          onClick$={() => count.value--}
          type="button"
          class="px-3 py-1 bg-red-500 text-white rounded hover:bg-red-600 transition-colors"
        >
          -
        </button>

        <span class="text-2xl font-bold text-blue-600 min-w-8 text-center">{count.value}</span>

        <button
          onClick$={() => count.value++}
          type="button"
          class="px-3 py-1 bg-green-500 text-white rounded hover:bg-green-600 transition-colors"
        >
          +
        </button>
      </div>

      <div class="text-sm text-gray-500">
        <p>• This component uses React hooks (useState)</p>
        <p>• It has event handlers (onClick)</p>
        <p>• It should run on the client, not the server</p>
      </div>
    </div>
  )
})
