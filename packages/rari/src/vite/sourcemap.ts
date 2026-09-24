export interface LineIdentitySourceMap {
  version: 3
  file: string
  sources: string[]
  sourcesContent: (string | null)[]
  names: string[]
  mappings: string
}

const VLQ_ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'
const MAX_LCS_CELLS = 2_000_000

export function encodeVlq(value: number): string {
  let vlq = value < 0 ? (-value << 1) + 1 : value << 1
  let encoded = ''
  do {
    let digit = vlq & 31
    vlq >>>= 5
    if (vlq > 0) digit |= 32
    encoded += VLQ_ALPHABET[digit]
  } while (vlq > 0)
  return encoded
}

export function encodeLineAlignedMappings(alignment: ReadonlyArray<number | null>): string {
  const lines: string[] = []
  let prevSourceLine = 0

  for (const origLine of alignment) {
    if (origLine == null) {
      lines.push('')
      continue
    }
    const relSourceLine = origLine - prevSourceLine
    prevSourceLine = origLine
    lines.push(`${encodeVlq(0)}${encodeVlq(0)}${encodeVlq(relSourceLine)}${encodeVlq(0)}`)
  }

  return lines.join(';')
}

export function lineIdentitySourceMap(id: string, sourceContent: string): LineIdentitySourceMap {
  const lineCount = Math.max(1, sourceContent.split('\n').length)
  const alignment = Array.from({ length: lineCount }, (_, index) => index)
  return {
    version: 3,
    file: id,
    sources: [id],
    sourcesContent: [sourceContent],
    names: [],
    mappings: encodeLineAlignedMappings(alignment),
  }
}

export function alignGeneratedLinesToOriginal(
  originalLines: readonly string[],
  generatedLines: readonly string[],
): Array<number | null> {
  const n = originalLines.length
  const m = generatedLines.length
  const result = Array.from<number | null>({ length: m }).fill(null)
  if (m === 0 || n === 0) return result

  if (linesEndWith(generatedLines, originalLines)) {
    const offset = m - n
    for (let i = 0; i < n; i += 1) result[offset + i] = i
    return result
  }

  if (linesStartWith(generatedLines, originalLines)) {
    for (let i = 0; i < n; i += 1) result[i] = i
    return result
  }

  if (n * m > MAX_LCS_CELLS) return heuristicAlign(originalLines, generatedLines)

  return alignWithLcs(originalLines, generatedLines)
}

function alignWithLcs(
  originalLines: readonly string[],
  generatedLines: readonly string[],
): Array<number | null> {
  const n = originalLines.length
  const m = generatedLines.length
  const result = Array.from<number | null>({ length: m }).fill(null)
  const dp = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1))
  for (let i = 1; i <= n; i += 1) {
    const oi = originalLines[i - 1]
    const row = dp[i]
    const prev = dp[i - 1]
    for (let j = 1; j <= m; j += 1) {
      row[j] = oi === generatedLines[j - 1] ? prev[j - 1] + 1 : Math.max(prev[j], row[j - 1])
    }
  }

  let i = n
  let j = m
  while (i > 0 && j > 0) {
    if (originalLines[i - 1] === generatedLines[j - 1]) {
      result[j - 1] = i - 1
      i -= 1
      j -= 1
    } else if (dp[i - 1][j] >= dp[i][j - 1]) {
      i -= 1
    } else {
      j -= 1
    }
  }

  return result
}

function linesEndWith(haystack: readonly string[], needle: readonly string[]): boolean {
  if (needle.length > haystack.length) return false
  const offset = haystack.length - needle.length
  for (let i = 0; i < needle.length; i += 1) {
    if (haystack[offset + i] !== needle[i]) return false
  }
  return true
}

function linesStartWith(haystack: readonly string[], needle: readonly string[]): boolean {
  if (needle.length > haystack.length) return false
  for (let i = 0; i < needle.length; i += 1) {
    if (haystack[i] !== needle[i]) return false
  }
  return true
}

function heuristicAlign(
  originalLines: readonly string[],
  generatedLines: readonly string[],
): Array<number | null> {
  const result = Array.from<number | null>({ length: generatedLines.length }).fill(null)
  let oi = 0
  let gi = 0
  while (oi < originalLines.length && gi < generatedLines.length) {
    if (originalLines[oi] === generatedLines[gi]) {
      result[gi] = oi
      oi += 1
      gi += 1
    } else {
      const skipGen = gi + 1 < generatedLines.length && originalLines[oi] === generatedLines[gi + 1]
      if (skipGen) {
        gi += 1
        continue
      }
      const skipOrig = oi + 1 < originalLines.length && originalLines[oi + 1] === generatedLines[gi]
      if (skipOrig) {
        oi += 1
        continue
      }
      gi += 1
      oi += 1
    }
  }
  return result
}

export function lineAlignedSourceMap(
  id: string,
  original: string,
  generated: string,
): LineIdentitySourceMap {
  const originalLines = original.split('\n')
  const generatedLines = generated.split('\n')
  const alignment = alignGeneratedLinesToOriginal(originalLines, generatedLines)

  return {
    version: 3,
    file: id,
    sources: [id],
    sourcesContent: [original],
    names: [],
    mappings: encodeLineAlignedMappings(alignment),
  }
}

export function emitTransformed(
  next: string,
  original: string,
  id: string,
): { code: string; map: LineIdentitySourceMap } | null {
  if (next === original) return null
  return {
    code: next,
    map: lineAlignedSourceMap(id, original, next),
  }
}
