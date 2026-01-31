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

export function logWarn(message: string) {
  console.warn(`${colors.yellow('⚠')} ${message}`)
}

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
