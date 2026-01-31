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

  if (cleaned.includes('||')) {
    const orParts = cleaned.split('||').map(part => part.trim())
    return orParts.some(part => isNodeVersionSufficient(part, minMajor))
  }

  const andParts = cleaned.split(/\s+(?:&&\s+)?/).filter(part => part && part !== '&&')
  if (andParts.length > 1) {
    for (const part of andParts) {
      const lowerBound = extractLowerBound(part, minMajor)
      if (lowerBound !== null && lowerBound >= minMajor)
        return true
    }

    return andParts.every(part => couldIncludeVersion(part, minMajor))
  }

  return extractMajorAndCompare(cleaned, minMajor)
}

function extractLowerBound(range: string, _minMajor: number): number | null {
  const match = range.match(/^>=?\s*(\d+)/)
  if (match)
    return Number.parseInt(match[1], 10)

  return null
}

function couldIncludeVersion(range: string, targetMajor: number): boolean {
  let match: RegExpMatchArray | null = null

  match = range.match(/^<=?\s*(\d+)/)
  if (match) {
    const upperMajor = Number.parseInt(match[1], 10)
    return targetMajor <= upperMajor
  }

  return extractMajorAndCompare(range, targetMajor)
}

function extractMajorAndCompare(versionRange: string, minMajor: number): boolean {
  let match: RegExpMatchArray | null = null

  match = versionRange.match(/^>=?\s*(\d+)\.(\d+)\.(\d+)/)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(/^<=?\s*(\d+)\.(\d+)\.(\d+)/)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(/^=?\s*(\d+)\.(\d+)\.(\d+)/)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(/^\^\s*(\d+)\.(\d+)\.(\d+)/)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(/^~\s*(\d+)\.(\d+)\.(\d+)/)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(/^(?:>=?|<=?|[=~^])?\s*(\d+)\.(\d+)/)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(/^(\d+)\.(?:x|\*)/i)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(/^(?:>=?|<=?|[=~^])\s*(\d+)(?:\s|$)/)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(/^(\d+)$/)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  return false
}
