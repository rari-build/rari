import colors from '@rari/colors'

export function logInfo(message: string) {
  console.warn(`${colors.blue('info')} ${message}`)
}

export function logSuccess(message: string) {
  console.warn(`${colors.green('✓')} ${message}`)
}

export function logError(message: string) {
  console.error(`${colors.red('✗')} ${message}`)
}

export function logWarning(message: string) {
  console.warn(`${colors.yellow('⚠')} ${message}`)
}

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
