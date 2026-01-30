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

  const [, major] = match
  const majorNum = Number.parseInt(major, 10)

  if (majorNum >= 22)
    return true

  return false
}
