export function clearTimer(timer: NodeJS.Timeout | null | undefined): null {
  if (timer) clearTimeout(timer)

  return null
}
