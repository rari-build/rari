import { getTodosList } from '../functions'

export default async function ShoppingList() {
  const groceries = await getTodosList()
  const timestamp = new Date().toLocaleTimeString()

  function renderGroceryItem(item: any) {
    const completedClass = item.completed
      ? 'px-4 py-2 border-b border-gray-200 relative line-through text-gray-500'
      : 'px-4 py-2 border-b border-gray-200 relative text-gray-800'

    const completedIndicator = item.completed
      ? (
          <span className="text-green-600">✓</span>
        )
      : null

    const leftDot = item.completed
      ? (
          <span className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-1 rounded-full bg-green-500 ml-1"></span>
        )
      : null

    return (
      <li key={item.id} className={completedClass}>
        {leftDot}
        {item.text}
        {' '}
        {completedIndicator}
      </li>
    )
  }

  return (
    <div className="p-5 rounded-lg" data-component-id="shoppinglist">
      <h1 className="text-2xl font-bold text-blue-700 mb-2">Shopping List</h1>
      <p className="text-gray-600 mb-4">A React Server Component demo</p>

      <ul className="space-y-2 mb-6">
        {groceries.map(item => renderGroceryItem(item))}
      </ul>

      <div className="mt-6 text-xs text-gray-500">
        <p className="mt-1">
          Server rendering time:
          {timestamp}
        </p>
      </div>
    </div>
  )
}
