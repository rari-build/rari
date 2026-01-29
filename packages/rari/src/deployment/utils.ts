export { logError, logInfo, logSuccess, logWarn } from '../logger'

export function isNodeVersionSufficient(versionRange: string): boolean {
  const cleaned = versionRange.trim()

  let match = cleaned.match(/^>=?\s*(\d+)\.(\d+)\.(\d+)/)
  if (!match)
    match = cleaned.match(/^[~^]\s*(\d+)\.(\d+)\.(\d+)/)
  if (!match)
    match = cleaned.match(/^(\d+)\.(\d+)\.(\d+)/)
  if (!match)
    match = cleaned.match(/^(\d+)\.(\d+)\.x/)
  if (!match)
    match = cleaned.match(/^(\d+)\.x/)

  if (!match)
    return false

  const [, major, minor] = match
  const majorNum = Number.parseInt(major, 10)
  const minorNum = minor ? Number.parseInt(minor, 10) : 0

  if (majorNum > 20)
    return true
  if (majorNum === 20 && minorNum >= 6)
    return true

  return false
}
