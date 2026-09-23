export async function sleep(ms: number) {
  return new Promise<void>(resolve => {
    setTimeout(resolve, ms)
  })
}

export function isoTimestamp(): string {
  return new Date().toISOString()
}
