export interface DebouncedFunc<T extends (...args: readonly any[]) => void> {
  (...args: Parameters<T>): void
  cancel: () => void
  flush: () => void
  pending: () => boolean
}

export function debounce<T extends (...args: readonly any[]) => void>(
  func: T,
  wait: number,
  options: Readonly<{
    leading?: boolean
    trailing?: boolean
    maxWait?: number
  }> = {},
): DebouncedFunc<T> {
  let timeoutId: ReturnType<typeof setTimeout> | null = null
  let lastCallTime = 0
  let lastInvokeTime = 0
  let lastArgs: Parameters<T> | null = null
  let lastThis: unknown = null

  const { leading = false, trailing = true, maxWait } = options

  function invokeFunc(time: number): void {
    const args = lastArgs!
    const thisArg = lastThis

    lastArgs = null
    lastThis = null
    lastInvokeTime = time

    func.apply(thisArg, args)
  }

  function shouldInvoke(time: number): boolean {
    const timeSinceLastCall = time - lastCallTime
    const timeSinceLastInvoke = time - lastInvokeTime

    return (
      lastCallTime === 0 ||
      timeSinceLastCall >= wait ||
      timeSinceLastCall < 0 ||
      (maxWait !== undefined && timeSinceLastInvoke >= maxWait)
    )
  }

  function timerExpired(): void {
    const time = Date.now()
    if (shouldInvoke(time)) {
      trailingEdge(time)
      return
    }
    const timeSinceLastCall = time - lastCallTime
    const timeSinceLastInvoke = time - lastInvokeTime
    const timeWaiting = wait - timeSinceLastCall
    const maxWaitRemaining = maxWait !== undefined ? maxWait - timeSinceLastInvoke : Infinity

    const remainingWait = Math.min(timeWaiting, maxWaitRemaining)
    timeoutId = setTimeout(timerExpired, remainingWait)
  }

  function leadingEdge(time: number): void {
    lastInvokeTime = time
    timeoutId = setTimeout(timerExpired, wait)
    if (leading) invokeFunc(time)
  }

  function trailingEdge(time: number): void {
    timeoutId = null

    if (trailing && lastArgs) {
      invokeFunc(time)
      return
    }
    lastArgs = null
    lastThis = null
  }

  function cancel(): void {
    if (timeoutId !== null) clearTimeout(timeoutId)
    lastInvokeTime = 0
    lastArgs = null
    lastCallTime = 0
    lastThis = null
    timeoutId = null
  }

  function flush(): void {
    if (timeoutId !== null) trailingEdge(Date.now())
  }

  function pending(): boolean {
    return timeoutId !== null
  }

  function debounced(this: unknown, ...args: Parameters<T>): void {
    const time = Date.now()
    const isInvoking = shouldInvoke(time)

    lastArgs = args
    // Preserve call-site `this` for delayed func.apply.
    // oxlint-disable-next-line typescript/no-this-alias
    lastThis = this
    lastCallTime = time

    if (isInvoking) {
      if (timeoutId === null) {
        leadingEdge(lastCallTime)
        return
      }
      /* v8 ignore start - edge case: isInvoking true with existing timeout and maxWait defined */
      if (maxWait !== undefined) {
        timeoutId = setTimeout(timerExpired, wait)
        invokeFunc(lastCallTime)
        return
      }
      /* v8 ignore stop */
    }
    timeoutId ??= setTimeout(timerExpired, wait)
  }

  debounced.cancel = cancel
  debounced.flush = flush
  debounced.pending = pending

  return debounced
}
