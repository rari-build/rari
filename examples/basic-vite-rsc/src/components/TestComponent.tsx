import { add } from '../functions'

interface TestComponentProps {
  a?: number
  b?: number
}

export default async function TestComponent({
  a = 5,
  b = 10,
}: TestComponentProps) {
  const result = await add(a, b)

  return (
    <div className="p-6 bg-white rounded-lg shadow-sm test-component">
      <h2 className="text-xl font-semibold text-gray-800 mb-3">
        Test Component
      </h2>
      <p className="text-gray-600 mb-4">
        This component is testing server function calls
      </p>
      <div className="p-4 bg-blue-50 border border-blue-100 rounded-md">
        <p className="text-gray-700">
          Server calculated:
          <span className="font-medium">{a}</span>
          {' + '}
          <span className="font-medium">{b}</span>
          {' = '}
          <span className="font-bold text-blue-600">{result}</span>
          <small className="ml-1 text-gray-500">(server)</small>
        </p>
        <small className="block mt-2 text-xs text-gray-500">
          Rendered at:
          {' '}
          {new Date().toLocaleTimeString()}
        </small>
      </div>
    </div>
  )
}
