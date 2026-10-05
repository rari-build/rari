import { component$, Resource, useResource$ } from '@qwik.dev/core'
import { add } from '../functions'

interface TestComponentProps {
  a?: number
  b?: number
}

export default component$((props: TestComponentProps) => {
  const a = props.a ?? 5
  const b = props.b ?? 10

  const result = useResource$(async () => {
    return add(a, b)
  })

  return (
    <Resource
      value={result}
      onResolved={value => (
        <div class="p-6 bg-white rounded-lg shadow-sm test-component">
          <h2 class="text-xl font-semibold text-gray-800 mb-3">Test Component</h2>
          <p class="text-gray-600 mb-4">This component is testing server function calls</p>
          <div class="p-4 bg-blue-50 border border-blue-100 rounded-md">
            <p class="text-gray-700">
              Server calculated:
              <span class="font-medium">{a}</span>
              {' + '}
              <span class="font-medium">{b}</span>
              {' = '}
              <span class="font-bold text-blue-600">{value}</span>
              <small class="ml-1 text-gray-500">(server)</small>
            </p>
            <small class="block mt-2 text-xs text-gray-500">
              Rendered at: {new Date().toLocaleTimeString()}
            </small>
          </div>
        </div>
      )}
    />
  )
})
