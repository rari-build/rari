export { logError, logInfo, logSuccess, logWarn } from '@rari/logger'

export const MIN_SUPPORTED_NODE_MAJOR = 22

const AND_SPLIT_REGEX = /\s+(?:&&\s+)?/
const LOWER_BOUND_REGEX = /^>=?\s*(\d+)/
const UPPER_BOUND_REGEX = /^<=?\s*(\d+)/
const UPPER_BOUND_ONLY_REGEX = /^<=?\s*\d+/
const SEMVER_RANGE_REGEX = /^>=?\s*(\d+)\.(\d+)\.(\d+)/
const EXACT_SEMVER_REGEX = /^=?\s*(\d+)\.(\d+)\.(\d+)/
const CARET_RANGE_REGEX = /^\^\s*(\d+)\.(\d+)\.(\d+)/
const TILDE_RANGE_REGEX = /^~\s*(\d+)\.(\d+)\.(\d+)/
const MAJOR_MINOR_REGEX = /^(?:>=?|<=?|[=~^])?\s*(\d+)\.(\d+)/
const WILDCARD_REGEX = /^(\d+)\.(?:x|\*)/i
const MAJOR_ONLY_REGEX = /^(?:>=?|[=~^])\s*(\d+)(?:\s|$)/
const NUMBER_ONLY_REGEX = /^(\d+)$/

export function isNodeVersionSufficient(versionRange: string, minMajor: number = MIN_SUPPORTED_NODE_MAJOR): boolean {
  const cleaned = versionRange.trim()

  if (cleaned.includes('||')) {
    const orParts = cleaned.split('||').map(part => part.trim())
    return orParts.some(part => isNodeVersionSufficient(part, minMajor))
  }

  const andParts = cleaned.split(AND_SPLIT_REGEX).filter(part => part && part !== '&&')
  if (andParts.length > 1) {
    for (const part of andParts) {
      const lowerBound = extractLowerBound(part)
      if (lowerBound !== null && lowerBound >= minMajor)
        return true
    }

    return andParts.every(part => couldIncludeVersion(part, minMajor))
  }

  return extractMajorAndCompare(cleaned, minMajor)
}

function extractLowerBound(range: string): number | null {
  const match = range.match(LOWER_BOUND_REGEX)
  if (match)
    return Number.parseInt(match[1], 10)

  return null
}

function couldIncludeVersion(range: string, targetMajor: number): boolean {
  let match: RegExpMatchArray | null = null

  match = range.match(UPPER_BOUND_REGEX)
  if (match) {
    const upperMajor = Number.parseInt(match[1], 10)
    return targetMajor <= upperMajor
  }

  return extractMajorAndCompare(range, targetMajor)
}

function extractMajorAndCompare(versionRange: string, minMajor: number): boolean {
  let match: RegExpMatchArray | null = null

  if (UPPER_BOUND_ONLY_REGEX.test(versionRange))
    return false

  match = versionRange.match(SEMVER_RANGE_REGEX)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(EXACT_SEMVER_REGEX)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(CARET_RANGE_REGEX)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(TILDE_RANGE_REGEX)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(MAJOR_MINOR_REGEX)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(WILDCARD_REGEX)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(MAJOR_ONLY_REGEX)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  match = versionRange.match(NUMBER_ONLY_REGEX)
  if (match) {
    const majorNum = Number.parseInt(match[1], 10)
    return majorNum >= minMajor
  }

  return false
}
