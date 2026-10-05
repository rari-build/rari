import process from 'node:process'
import { component$ } from '@qwik.dev/core'

export default component$(() => {
  const nodeEnv = process?.env?.NODE_ENV
  const serverPort = process?.env?.SERVER_PORT
  const customVar = process?.env?.CUSTOM_TEST_VAR

  return (
    <div class="p-6 bg-white rounded-lg shadow-sm">
      <h2 class="text-xl font-semibold text-gray-800 mb-4">Environment Variables Test</h2>

      <div class="space-y-4">
        <div class="p-4 bg-gray-50 border border-gray-200 rounded">
          <h3 class="font-medium text-gray-700 mb-2">Process Object</h3>
          <p>
            process available:
            {typeof process !== 'undefined' ? 'Yes' : 'No'}
          </p>
          <p>
            process.env available:
            {typeof process?.env !== 'undefined' ? 'Yes' : 'No'}
          </p>
        </div>

        <div class="p-4 bg-blue-50 border border-blue-200 rounded">
          <h3 class="font-medium text-gray-700 mb-2">Environment Variables</h3>
          <div class="space-y-1 text-sm">
            <p>
              <span class="font-mono">NODE_ENV</span>: {nodeEnv || 'undefined'}
            </p>
            <p>
              <span class="font-mono">SERVER_PORT</span>: {serverPort || 'undefined'}
            </p>
            <p>
              <span class="font-mono">CUSTOM_TEST_VAR</span>: {customVar || 'undefined'}
            </p>
          </div>
        </div>

        <div class="p-4 bg-yellow-50 border border-yellow-200 rounded">
          <h3 class="font-medium text-gray-700 mb-2">Raw process.env</h3>
          <pre class="text-xs bg-white p-2 rounded border overflow-auto max-h-32">
            {JSON.stringify(process?.env || {}, null, 2)}
          </pre>
        </div>

        <div class="text-xs text-gray-500 mt-4">Rendered at: {new Date().toISOString()}</div>
      </div>
    </div>
  )
})
