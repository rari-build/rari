'use client'

import {
  Activity,
  Fragment,
  startTransition,
  useEffect,
  useRef,
  useState,
  ViewTransition,
} from 'react'

const CARDS = [
  { id: 'alpha', label: 'Alpha', detail: 'First sibling in the fragment' },
  { id: 'beta', label: 'Beta', detail: 'Second sibling, no wrapper div' },
  { id: 'gamma', label: 'Gamma', detail: 'Third sibling shares the FragmentInstance' },
] as const

export function FragmentRefsDemo() {
  const fragmentRef = useRef<React.FragmentInstance | null>(null)
  const [visibleIds, setVisibleIds] = useState<readonly string[]>([])

  useEffect(() => {
    const instance = fragmentRef.current
    if (instance == null) return undefined

    const observer = new IntersectionObserver(
      entries => {
        const next = entries
          .filter(entry => entry.isIntersecting)
          .map(entry => {
            if (!(entry.target instanceof HTMLElement)) return null
            return entry.target.dataset.cardId ?? null
          })
          .filter((id): id is string => id != null && id !== '')
        setVisibleIds(next)
      },
      { threshold: 0.6 },
    )

    instance.observeUsing(observer)
    return () => {
      instance.unobserveUsing(observer)
      observer.disconnect()
    }
  }, [])

  return (
    <div className="space-y-4">
      <p className="text-sm text-gray-600">
        A <code>ref</code> on <code>&lt;Fragment&gt;</code> yields a <code>FragmentInstance</code>:
        observe, focus, and measure sibling DOM nodes without an extra wrapper element.
      </p>

      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          className="px-3 py-1.5 text-sm font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800"
          onClick={() => {
            fragmentRef.current?.focus({ preventScroll: true })
          }}
        >
          Focus first card
        </button>
        <button
          type="button"
          className="px-3 py-1.5 text-sm font-medium rounded-md border border-gray-300 text-gray-800 hover:bg-gray-50"
          onClick={() => {
            fragmentRef.current?.focusLast({ preventScroll: true })
          }}
        >
          Focus last card
        </button>
      </div>

      <p className="text-xs text-gray-500" data-testid="fragment-visible-ids">
        Visible: {visibleIds.length > 0 ? visibleIds.join(', ') : 'none yet'}
      </p>

      <div className="max-h-40 overflow-y-auto space-y-2 rounded-lg border border-gray-200 p-3 bg-gray-50">
        <Fragment ref={fragmentRef}>
          {CARDS.map(card => (
            <button
              key={card.id}
              type="button"
              data-card-id={card.id}
              className="block w-full text-left rounded-md border border-gray-200 bg-white px-3 py-3 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <span className="font-semibold text-gray-900">{card.label}</span>
              <span className="mt-1 block text-sm text-gray-600">{card.detail}</span>
            </button>
          ))}
        </Fragment>
      </div>
    </div>
  )
}

export function ActivityDemo() {
  const [showPanel, setShowPanel] = useState(true)
  const [count, setCount] = useState(0)

  return (
    <div className="space-y-4">
      <p className="text-sm text-gray-600">
        <code>Activity</code> keeps the panel mounted while hidden so state survives. Paired with{' '}
        <code>ViewTransition</code> for enter/exit.
      </p>

      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          className="px-3 py-1.5 text-sm font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800"
          onClick={() => {
            startTransition(() => {
              setShowPanel(current => !current)
            })
          }}
        >
          {showPanel ? 'Hide panel' : 'Show panel'}
        </button>
        <button
          type="button"
          className="px-3 py-1.5 text-sm font-medium rounded-md border border-gray-300 text-gray-800 hover:bg-gray-50"
          onClick={() => {
            setCount(current => current + 1)
          }}
          disabled={!showPanel}
        >
          Increment
        </button>
      </div>

      <Activity mode={showPanel ? 'visible' : 'hidden'}>
        <ViewTransition enter="auto" exit="auto" default="none">
          <div
            className="rounded-lg border border-indigo-200 bg-indigo-50 p-4"
            data-testid="activity-panel"
          >
            <p className="text-sm text-indigo-900">
              Counter stays alive while hidden:{' '}
              <span className="font-semibold" data-testid="activity-count">
                {count}
              </span>
            </p>
          </div>
        </ViewTransition>
      </Activity>
    </div>
  )
}
