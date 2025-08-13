'use client'

import { useState } from 'react'

import EnvTestComponent from './EnvTestComponent'
import FetchExample from './FetchExample'
import Markdown from './Markdown'
import ServerWithClient from './ServerWithClient'
import ShoppingList from './ShoppingList'
import SimpleStreamingTest from './SimpleStreamingTest'
import StreamingVerificationTest from './StreamingVerificationTest'
import StressTest from './StressTest'
import SuspenseStreamingTest from './SuspenseStreamingTest'
import TestComponent from './TestComponent'
import WhatsHot from './WhatsHot'

interface ServerComponent {
  id: string
  name: string
}

interface ComponentSelectorProps {
  serverComponents: ServerComponent[]
}

const componentMap = {
  ServerWithClient,
  EnvTestComponent,
  ShoppingList,
  FetchExample,
  WhatsHot,
  TestComponent,
  Markdown,
  SimpleStreamingTest,
  StressTest,
  StreamingVerificationTest,
  SuspenseStreamingTest,
} as const

export default function ComponentSelector({
  serverComponents,
}: ComponentSelectorProps) {
  const [activeComponent, setActiveComponent]
    = useState<string>('ServerWithClient')

  const ComponentToRender
    = componentMap[activeComponent as keyof typeof componentMap]

  return (
    <>
      <div className="mb-6 flex flex-wrap gap-2 justify-center">
        {serverComponents.map(comp => (
          <button
            key={comp.id}
            onClick={() => setActiveComponent(comp.id)}
            type="button"
            className={`px-4 py-2 rounded-lg font-medium transition-colors ${
              activeComponent === comp.id
                ? 'bg-blue-600 text-white'
                : 'bg-white text-gray-700 hover:bg-gray-50 border border-gray-300'
            }`}
          >
            {comp.name}
          </button>
        ))}
      </div>

      <main className="bg-white rounded-lg p-6 shadow-sm">
        <h2 className="text-xl font-semibold mb-4">{activeComponent}</h2>

        <div className="min-h-[200px]">
          {ComponentToRender
            ? (
                <ComponentToRender key={activeComponent} />
              )
            : (
                <div>Component not found</div>
              )}
        </div>

        <div className="mt-8 p-4 bg-blue-50 border border-blue-200 rounded-lg">
          <h3 className="font-semibold text-blue-900 mb-2">
            RSC Architecture Demo
          </h3>
          <div className="text-sm text-blue-800 space-y-1">
            <div>
              •
              {' '}
              <strong>App.tsx</strong>
              : Server Component (renders server
              components)
            </div>
            <div>
              •
              {' '}
              <strong>ComponentSelector.tsx</strong>
              : Client Component
              (handles interactivity)
            </div>
            <div>
              •
              {' '}
              <strong>Server Components</strong>
              : Run on server, can fetch
              data, no interactivity
            </div>
            <div>
              •
              {' '}
              <strong>Client Components</strong>
              : Run in browser, have state
              and event handlers
            </div>
            <div className="mt-2 text-xs">
              Current:
              {' '}
              <span className="font-mono bg-blue-100 px-1 rounded">
                Server Component
              </span>
            </div>
          </div>
        </div>
      </main>
    </>
  )
}
