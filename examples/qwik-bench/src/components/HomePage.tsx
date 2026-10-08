import { component$ } from '@qwik.dev/core'
import Counter from './Counter'
import EnvTestComponent from './EnvTestComponent'
import FetchExample from './FetchExample'
import Markdown from './Markdown'
import ServerWithClient from './ServerWithClient'
import ShoppingList from './ShoppingList'
import TestComponent from './TestComponent'
import WhatsHot from './WhatsHot'

interface HomePageProps {
  framework: {
    name: string
    emoji: string
    runtime: string
    runtimeColor: string
    feature1: string
    feature1Color: string
    feature2: string
    feature2Color: string
    poweredBy: string
  }
}

export default component$((props: HomePageProps) => {
  const { framework } = props

  return (
    <div class="min-h-screen bg-linear-to-br from-blue-50 to-indigo-100 py-8 px-4">
      <div class="max-w-6xl mx-auto space-y-8">
        <div class="bg-white rounded-xl p-8 shadow-sm border border-gray-200 text-center">
          <h1 class="text-4xl font-bold text-gray-900 mb-4">
            {framework.emoji} {framework.name} Framework Benchmark
          </h1>
          <p class="text-xl text-gray-600 mb-6">Server Component Performance Testing Suite</p>
          <div
            class={`grid grid-cols-1 md:grid-cols-3 gap-4 ${framework.runtimeColor} p-6 rounded-lg`}
          >
            <div class="text-center">
              <div
                class={`text-2xl font-bold ${framework.runtimeColor.replace('bg-', 'text-').replace('-50', '-600')}`}
              >
                {framework.runtime}
              </div>
              <div
                class={`text-sm ${framework.runtimeColor.replace('bg-', 'text-').replace('-50', '-700')}`}
              >
                Runtime Engine
              </div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-purple-600">React</div>
              <div class="text-sm text-purple-700">Server Components</div>
            </div>
            <div class="text-center">
              <div class={`text-2xl font-bold ${framework.feature2Color}`}>
                {framework.feature2}
              </div>
              <div
                class={`text-sm ${framework.feature2Color.replace('text-', 'text-').replace('-600', '-700')}`}
              >
                {framework.feature1}
              </div>
            </div>
          </div>
        </div>

        <div class="bg-white rounded-xl p-8 shadow-sm border border-gray-200">
          <h2 class="text-2xl font-bold text-gray-900 mb-6 text-center">
            🎯 Server Components Showcase
          </h2>
          <p class="text-gray-600 text-center mb-8">
            All components rendered server-side for optimal performance benchmarking
          </p>

          <div class="grid gap-6 md:grid-cols-2 lg:grid-cols-2">
            <div class="bg-gray-50 p-6 rounded-lg border border-gray-200">
              <h3 class="text-lg font-semibold text-gray-900 mb-4 text-center">Counter</h3>
              <Counter />
            </div>

            <div class="bg-gray-50 p-6 rounded-lg border border-gray-200">
              <h3 class="text-lg font-semibold text-gray-900 mb-4 text-center">Test Component</h3>
              <TestComponent />
            </div>

            <div class="bg-gray-50 p-6 rounded-lg border border-gray-200">
              <h3 class="text-lg font-semibold text-gray-900 mb-4 text-center">Shopping List</h3>
              <ShoppingList />
            </div>

            <div class="bg-gray-50 p-6 rounded-lg border border-gray-200">
              <h3 class="text-lg font-semibold text-gray-900 mb-4 text-center">What's Hot</h3>
              <WhatsHot />
            </div>

            <div class="bg-gray-50 p-6 rounded-lg border border-gray-200">
              <h3 class="text-lg font-semibold text-gray-900 mb-4 text-center">Environment Test</h3>
              <EnvTestComponent />
            </div>

            <div class="bg-gray-50 p-6 rounded-lg border border-gray-200">
              <h3 class="text-lg font-semibold text-gray-900 mb-4 text-center">Fetch Example</h3>
              <FetchExample />
            </div>

            <div class="bg-gray-50 p-6 rounded-lg border border-gray-200">
              <h3 class="text-lg font-semibold text-gray-900 mb-4 text-center">
                Server with Client
              </h3>
              <ServerWithClient />
            </div>

            <div class="bg-gray-50 p-6 rounded-lg border border-gray-200">
              <h3 class="text-lg font-semibold text-gray-900 mb-4 text-center">
                Markdown Renderer
              </h3>
              <Markdown />
            </div>
          </div>
        </div>

        <div class="bg-linear-to-r from-green-50 to-blue-50 rounded-xl p-8 border border-green-200">
          <h2 class="text-2xl font-bold text-gray-900 mb-4">📈 Benchmark Summary</h2>
          <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            <div class="bg-white p-4 rounded-lg text-center">
              <div class="text-2xl font-bold text-blue-600">8</div>
              <div class="text-sm text-gray-700">Components</div>
            </div>
            <div class="bg-white p-4 rounded-lg text-center">
              <div class="text-2xl font-bold text-green-600">Static</div>
              <div class="text-sm text-gray-700">Rendering</div>
            </div>
            <div class="bg-white p-4 rounded-lg text-center">
              <div class="text-2xl font-bold text-purple-600">Server</div>
              <div class="text-sm text-gray-700">Components</div>
            </div>
            <div class="bg-white p-4 rounded-lg text-center">
              <div class="text-2xl font-bold text-orange-600">Pure</div>
              <div class="text-sm text-gray-700">SSR</div>
            </div>
          </div>
          <div class="mt-6 text-center">
            <p class="text-gray-600">
              Static server component rendering for accurate performance benchmarking.
              <br />
              <span class="text-sm text-gray-500">Powered by {framework.poweredBy}</span>
            </p>
          </div>
        </div>
      </div>
    </div>
  )
})
