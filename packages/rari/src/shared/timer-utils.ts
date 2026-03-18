export function clearTimer<T extends NodeJS.Timeout | null | undefined>(
  timer: T,
): null {
  if (timer) {
    clearTimeout(timer)
  }

  return null
}
