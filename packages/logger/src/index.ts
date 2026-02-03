import { styleText } from 'node:util'

export function logInfo(message: string) {
  console.warn(`${styleText('blue', 'info')} ${message}`)
}

export function logSuccess(message: string) {
  console.warn(`${styleText('green', '✓')} ${message}`)
}

export function logError(message: string) {
  console.error(`${styleText('red', '✗')} ${message}`)
}

export function logWarn(message: string) {
  console.warn(`${styleText('yellow', '⚠')} ${message}`)
}
