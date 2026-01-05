export interface DebouncedFunc<T extends (...args: any[]) => any> {
  (...args: Parameters<T>): void
  cancel: () => void
  flush: () => void
  pending: () => boolean
}

export function debounce<T extends (...args: any[]) => any>(
  func: T,
  wait: number,
  options: {
    leading?: boolean
    trailing?: boolean
    maxWait?: number
  } = {},
): DebouncedFunc<T> {
  let timeoutId: ReturnType<typeof setTimeout> | null = null
  let lastCallTime = 0
  let lastInvokeTime = 0
  let lastArgs: Parameters<T> | null = null
  let lastThis: any = null

  const { leading = false, trailing = true, maxWait } = options

  function invokeFunc(time: number) {
    const args = lastArgs!
    const thisArg = lastThis

    lastArgs = null
    lastThis = null
    lastInvokeTime = time

    return func.apply(thisArg, args)
  }

  function shouldInvoke(time: number): boolean {
    const timeSinceLastCall = time - lastCallTime
    const timeSinceLastInvoke = time - lastInvokeTime

    return (
      lastCallTime === 0
      || timeSinceLastCall >= wait
      || timeSinceLastCall < 0
      || (maxWait !== undefined && timeSinceLastInvoke >= maxWait)
    )
  }

  function timerExpired() {
    const time = Date.now()
    if (shouldInvoke(time)) {
      return trailingEdge(time)
    }
    const timeSinceLastCall = time - lastCallTime
    const timeSinceLastInvoke = time - lastInvokeTime
    const timeWaiting = wait - timeSinceLastCall
    const maxWaitRemaining = maxWait !== undefined ? maxWait - timeSinceLastInvoke : Infinity

    const remainingWait = Math.min(timeWaiting, maxWaitRemaining)
    timeoutId = setTimeout(timerExpired, remainingWait)
  }

  function leadingEdge(time: number) {
    lastInvokeTime = time
    timeoutId = setTimeout(timerExpired, wait)
    return leading ? invokeFunc(time) : undefined
  }

  function trailingEdge(time: number) {
    timeoutId = null

    if (trailing && lastArgs)
      return invokeFunc(time)
    lastArgs = null
    lastThis = null
    return undefined
  }

  function cancel() {
    if (timeoutId !== null)
      clearTimeout(timeoutId)
    lastInvokeTime = 0
    lastArgs = null
    lastCallTime = 0
    lastThis = null
    timeoutId = null
  }

  function flush() {
    return timeoutId === null ? undefined : trailingEdge(Date.now())
  }

  function pending() {
    return timeoutId !== null
  }

  function debounced(this: any, ...args: Parameters<T>) {
    const time = Date.now()
    const isInvoking = shouldInvoke(time)

    lastArgs = args
    lastThis = this
    lastCallTime = time

    if (isInvoking) {
      if (timeoutId === null) {
        return leadingEdge(lastCallTime)
      }
      if (maxWait !== undefined) {
        timeoutId = setTimeout(timerExpired, wait)
        return invokeFunc(lastCallTime)
      }
    }
    if (timeoutId === null) {
      timeoutId = setTimeout(timerExpired, wait)
    }
    return undefined
  }

  debounced.cancel = cancel
  debounced.flush = flush
  debounced.pending = pending

  return debounced
}
