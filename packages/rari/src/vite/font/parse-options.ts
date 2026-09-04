export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue }

export function extractObjectLiteral(
  source: string,
  openBraceIndex: number,
): { value: Record<string, JsonValue>; end: number } | null {
  if (source[openBraceIndex] !== '{') return null
  try {
    const { json, end } = readValue(source, openBraceIndex)
    if (json == null || typeof json !== 'object' || Array.isArray(json)) return null
    return { value: json, end }
  } catch {
    return null
  }
}

function skipWs(source: string, index: number): number {
  let i = index
  while (i < source.length && /\s/.test(source[i] ?? '')) i += 1
  return i
}

function readValue(source: string, index: number): { json: JsonValue; end: number } {
  const i = skipWs(source, index)
  const ch = source[i]
  if (ch === '{') return readObject(source, i)
  if (ch === '[') return readArray(source, i)
  if (ch === "'" || ch === '"') return readString(source, i)
  if (ch === '-' || (ch >= '0' && ch <= '9')) return readNumber(source, i)
  if (source.startsWith('true', i)) return { json: true, end: i + 4 }
  if (source.startsWith('false', i)) return { json: false, end: i + 5 }
  if (source.startsWith('null', i)) return { json: null, end: i + 4 }
  throw new Error(`Unsupported token at ${i}`)
}

function readString(source: string, index: number): { json: string; end: number } {
  const quote = source[index]
  let i = index + 1
  let out = ''
  while (i < source.length) {
    const ch = source[i]
    if (ch === '\\') {
      if (i + 1 >= source.length) throw new Error('Unterminated string escape')
      out += source[i + 1]
      i += 2
      continue
    }
    if (ch === quote) return { json: out, end: i + 1 }
    out += ch
    i += 1
  }
  throw new Error('Unterminated string')
}

function readNumber(source: string, index: number): { json: number; end: number } {
  const match = /^-?\d+(?:\.\d+)?(?:e[+-]?\d+)?/i.exec(source.slice(index))
  if (match == null) throw new Error('Invalid number')
  return { json: Number(match[0]), end: index + match[0].length }
}

function readArray(source: string, index: number): { json: JsonValue[]; end: number } {
  const items: JsonValue[] = []
  let i = skipWs(source, index + 1)
  if (source[i] === ']') return { json: items, end: i + 1 }
  while (i < source.length) {
    const item = readValue(source, i)
    items.push(item.json)
    i = skipWs(source, item.end)
    if (source[i] === ',') {
      i = skipWs(source, i + 1)
      if (source[i] === ']') return { json: items, end: i + 1 }
      continue
    }
    if (source[i] === ']') return { json: items, end: i + 1 }
    throw new Error('Expected comma or ]')
  }
  throw new Error('Unterminated array')
}

function readObject(
  source: string,
  index: number,
): { json: Record<string, JsonValue>; end: number } {
  const obj: Record<string, JsonValue> = {}
  let i = skipWs(source, index + 1)
  if (source[i] === '}') return { json: obj, end: i + 1 }
  while (i < source.length) {
    let key: string
    if (source[i] === "'" || source[i] === '"') {
      const parsed = readString(source, i)
      key = parsed.json
      i = skipWs(source, parsed.end)
    } else {
      const match = /^[A-Z_$][\w$]*/i.exec(source.slice(i))
      if (match == null) throw new Error('Expected property name')
      key = match[0]
      i = skipWs(source, i + key.length)
    }
    if (source[i] !== ':') throw new Error('Expected :')
    const value = readValue(source, i + 1)
    obj[key] = value.json
    i = skipWs(source, value.end)
    if (source[i] === ',') {
      i = skipWs(source, i + 1)
      if (source[i] === '}') return { json: obj, end: i + 1 }
      continue
    }
    if (source[i] === '}') return { json: obj, end: i + 1 }
    throw new Error('Expected comma or }')
  }
  throw new Error('Unterminated object')
}
