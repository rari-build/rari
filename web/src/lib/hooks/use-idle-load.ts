import { useEffect, useEffectEvent } from 'react'

const IDLE_EVENTS = ['click', 'scroll', 'keydown'] as const

export function useIdleLoad(onIdle: () => void, delayMs: number): void {
  const onIdleEvent = useEffectEvent(onIdle)

  useEffect(() => {
    let started = false

    const run = () => {
      if (started) return
      started = true
      onIdleEvent()
      for (const event of IDLE_EVENTS) {
        document.removeEventListener(event, run)
      }
    }

    for (const event of IDLE_EVENTS) {
      document.addEventListener(event, run, { once: true, passive: true })
    }
    const timer = setTimeout(run, delayMs)

    return () => {
      clearTimeout(timer)
      for (const event of IDLE_EVENTS) {
        document.removeEventListener(event, run)
      }
    }
  }, [delayMs])
}
