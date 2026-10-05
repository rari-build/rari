import { component$, Resource, useResource$ } from '@qwik.dev/core'
import { getTodosList } from '../functions'

function renderGroceryItem(item: any) {
  const completedClass = item.completed
    ? 'px-4 py-2 border-b border-gray-200 relative line-through text-gray-500'
    : 'px-4 py-2 border-b border-gray-200 relative text-gray-800'

  const completedIndicator = item.completed ? <span class="text-green-600">✓</span> : null

  const leftDot = item.completed ? (
    <span class="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-1 rounded-full bg-green-500 ml-1"></span>
  ) : null

  return (
    <li key={item.id} class={completedClass}>
      {leftDot}
      {item.text} {completedIndicator}
    </li>
  )
}

export default component$(() => {
  const groceriesResource = useResource$(async () => {
    return getTodosList()
  })

  return (
    <Resource
      value={groceriesResource}
      onResolved={groceries => {
        const timestamp = new Date().toLocaleTimeString()

        return (
          <div class="p-5 rounded-lg" data-component-id="shoppinglist">
            <h1 class="text-2xl font-bold text-blue-700 mb-2">Shopping List</h1>
            <p class="text-gray-600 mb-4">A React Server Component demo</p>

            <ul class="space-y-2 mb-6">{groceries.map(item => renderGroceryItem(item))}</ul>

            <div class="mt-6 text-xs text-gray-500">
              <p class="mt-1">
                Server rendering time:
                {timestamp}
              </p>
            </div>
          </div>
        )
      }}
    />
  )
})
