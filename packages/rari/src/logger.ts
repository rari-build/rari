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

export function logWarning(message: string) {
  console.warn(`${colors.yellow('⚠')} ${message}`)
}
