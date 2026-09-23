export function isoTimestamp(): string {
  return new Date().toISOString()
}

export function localTimestamp(): string {
  return new Date().toLocaleTimeString()
}
