import colors from '@rari/colors'

export const MIN_SUPPORTED_NODE_MAJOR = 22

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

export function isNodeVersionSufficient(versionRange: string, minMajor: number = MIN_SUPPORTED_NODE_MAJOR): boolean {
  const cleaned = versionRange.trim()

  let match = cleaned.match(/^>=?\s*(\d+)\.(\d+)\.(\d+)/)
  if (!match)
    match = cleaned.match(/^[~^]\s*(\d+)\.(\d+)\.(\d+)/)
  if (!match)
    match = cleaned.match(/^(\d+)\.(\d+)\.(\d+)/)
  if (!match)
    match = cleaned.match(/^(\d+)\.(\d+)\.x/)
  if (!match)
    match = cleaned.match(/^>=?\s*(\d+)(?:\.\d+)?(?:\.\d+)?$/)
  if (!match)
    match = cleaned.match(/^[~^]\s*(\d+)(?:\.\d+)?(?:\.\d+)?$/)
  if (!match)
    match = cleaned.match(/^(\d+)\.x/)
  if (!match)
    return false

  const [, major] = match
  const majorNum = Number.parseInt(major, 10)

  return majorNum >= minMajor
}
