import { component$ } from '@qwik.dev/core'
import HomePage from '../components/HomePage'

export default component$(() => {
  return (
    <HomePage
      framework={{
        name: 'rari + Qwik',
        emoji: '🦀',
        runtime: 'Rust + Resumable',
        runtimeColor: 'bg-blue-50',
        feature1: 'Configuration',
        feature1Color: 'text-green-700',
        feature2: 'Zero',
        feature2Color: 'text-green-600',
        poweredBy: "rari's Rust host + Qwik resumability",
      }}
    />
  )
})
