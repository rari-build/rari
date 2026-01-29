export { logError, logInfo, logSuccess, logWarn, logWarning } from '../logger'

export function isNodeVersionSufficient(versionRange: string): boolean {
  const match = versionRange.match(/(\d+)\.(\d+)\.(\d+)/)
  if (!match)
    return true

  const [, major, minor] = match
  const majorNum = Number.parseInt(major, 10)
  const minorNum = Number.parseInt(minor, 10)

  if (majorNum > 20)
    return true
  if (majorNum === 20 && minorNum >= 6)
    return true

  return false
}
