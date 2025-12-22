type FrontmatterValue = string | number | boolean | null | FrontmatterValue[] | { [key: string]: FrontmatterValue }

interface ParsedContent {
  data: Record<string, FrontmatterValue>
  content: string
}

export function parseFrontmatter(content: string): ParsedContent {
  const frontmatterRegex = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/
  const match = content.match(frontmatterRegex)

  if (!match) {
    return {
      data: {},
      content,
    }
  }

  const [, frontmatterText, markdownContent] = match
  const data = parseYaml(frontmatterText)

  return {
    data,
    content: markdownContent,
  }
}

function parseYaml(yaml: string): Record<string, FrontmatterValue> {
  const lines = yaml.split(/\r?\n/)
  const result: Record<string, FrontmatterValue> = {}
  let i = 0

  while (i < lines.length) {
    const line = lines[i]

    if (!line.trim() || line.trim().startsWith('#')) {
      i++
      continue
    }

    const colonIndex = line.indexOf(':')
    if (colonIndex === -1 || line.startsWith(' ') || line.startsWith('-')) {
      i++
      continue
    }

    const indent = line.length - line.trimStart().length
    const key = line.substring(0, colonIndex).trim()
    const valueStart = line.substring(colonIndex + 1).trim()

    if (!valueStart) {
      const { value, nextIndex } = parseComplexValue(lines, i + 1, indent)
      result[key] = value
      i = nextIndex
    }
    else if (valueStart === '|' || valueStart === '>') {
      const { value, nextIndex } = parseMultilineString(lines, i + 1, indent, valueStart === '>')
      result[key] = value
      i = nextIndex
    }
    else if (valueStart.startsWith('[')) {
      result[key] = parseInlineArray(valueStart)
      i++
    }
    else if (valueStart.startsWith('{')) {
      result[key] = parseInlineObject(valueStart)
      i++
    }
    else {
      result[key] = parseValue(valueStart)
      i++
    }
  }

  return result
}

function parseComplexValue(lines: string[], startIndex: number, parentIndent: number): { value: FrontmatterValue, nextIndex: number } {
  if (startIndex >= lines.length) {
    return { value: null, nextIndex: startIndex }
  }

  const firstLine = lines[startIndex]
  const firstLineIndent = firstLine.length - firstLine.trimStart().length

  if (firstLine.trim().startsWith('-')) {
    return parseArray(lines, startIndex, parentIndent)
  }

  if (firstLineIndent > parentIndent && firstLine.includes(':')) {
    return parseObject(lines, startIndex, parentIndent)
  }

  return { value: null, nextIndex: startIndex }
}

function parseArray(lines: string[], startIndex: number, parentIndent: number): { value: FrontmatterValue[], nextIndex: number } {
  const array: FrontmatterValue[] = []
  let i = startIndex

  while (i < lines.length) {
    const line = lines[i]
    const indent = line.length - line.trimStart().length
    const trimmed = line.trim()

    if (indent <= parentIndent && trimmed) {
      break
    }

    if (!trimmed || trimmed.startsWith('#')) {
      i++
      continue
    }

    if (trimmed.startsWith('-')) {
      const valueStart = trimmed.substring(1).trim()

      if (!valueStart) {
        const { value, nextIndex } = parseComplexValue(lines, i + 1, indent)
        array.push(value)
        i = nextIndex
      }
      else if (valueStart.startsWith('[')) {
        array.push(parseInlineArray(valueStart))
        i++
      }
      else if (valueStart.startsWith('{')) {
        array.push(parseInlineObject(valueStart))
        i++
      }
      else {
        array.push(parseValue(valueStart))
        i++
      }
    }
    else {
      i++
    }
  }

  return { value: array, nextIndex: i }
}

function parseObject(lines: string[], startIndex: number, parentIndent: number): { value: Record<string, FrontmatterValue>, nextIndex: number } {
  const obj: Record<string, FrontmatterValue> = {}
  let i = startIndex

  while (i < lines.length) {
    const line = lines[i]
    const indent = line.length - line.trimStart().length
    const trimmed = line.trim()

    if (indent <= parentIndent && trimmed) {
      break
    }

    if (!trimmed || trimmed.startsWith('#')) {
      i++
      continue
    }

    const colonIndex = line.indexOf(':')
    if (colonIndex === -1) {
      i++
      continue
    }

    const key = line.substring(0, colonIndex).trim()
    const valueStart = line.substring(colonIndex + 1).trim()

    if (!valueStart) {
      const { value, nextIndex } = parseComplexValue(lines, i + 1, indent)
      obj[key] = value
      i = nextIndex
    }
    else if (valueStart === '|' || valueStart === '>') {
      const { value, nextIndex } = parseMultilineString(lines, i + 1, indent, valueStart === '>')
      obj[key] = value
      i = nextIndex
    }
    else if (valueStart.startsWith('[')) {
      obj[key] = parseInlineArray(valueStart)
      i++
    }
    else if (valueStart.startsWith('{')) {
      obj[key] = parseInlineObject(valueStart)
      i++
    }
    else {
      obj[key] = parseValue(valueStart)
      i++
    }
  }

  return { value: obj, nextIndex: i }
}

function parseMultilineString(lines: string[], startIndex: number, parentIndent: number, folded: boolean): { value: string, nextIndex: number } {
  const stringLines: string[] = []
  let i = startIndex

  while (i < lines.length) {
    const line = lines[i]
    const indent = line.length - line.trimStart().length

    if (indent <= parentIndent && line.trim()) {
      break
    }

    if (indent > parentIndent) {
      stringLines.push(line.substring(parentIndent + 2))
    }

    i++
  }

  if (folded) {
    return { value: stringLines.join(' ').trim(), nextIndex: i }
  }
  else {
    return { value: stringLines.join('\n'), nextIndex: i }
  }
}

function parseInlineArray(str: string): FrontmatterValue[] {
  const content = str.substring(1, str.lastIndexOf(']')).trim()
  if (!content)
    return []

  return content.split(',').map(item => parseValue(item.trim()))
}

function parseInlineObject(str: string): Record<string, FrontmatterValue> {
  const content = str.substring(1, str.lastIndexOf('}')).trim()
  const obj: Record<string, FrontmatterValue> = {}

  if (!content)
    return obj

  const pairs = content.split(',')
  for (const pair of pairs) {
    const colonIndex = pair.indexOf(':')
    if (colonIndex > 0) {
      const key = pair.substring(0, colonIndex).trim().replace(/^["']|["']$/g, '')
      const value = pair.substring(colonIndex + 1).trim()
      obj[key] = parseValue(value)
    }
  }

  return obj
}

function parseValue(str: string): FrontmatterValue {
  const trimmed = str.trim()

  if (trimmed === 'null' || trimmed === '~' || trimmed === '') {
    return null
  }

  if (trimmed === 'true' || trimmed === 'yes' || trimmed === 'on') {
    return true
  }
  if (trimmed === 'false' || trimmed === 'no' || trimmed === 'off') {
    return false
  }

  if (/^-?\d+$/.test(trimmed)) {
    return Number.parseInt(trimmed, 10)
  }
  if (/^-?\d+\.\d+$/.test(trimmed)) {
    return Number.parseFloat(trimmed)
  }

  if ((trimmed.startsWith('"') && trimmed.endsWith('"'))
    || (trimmed.startsWith('\'') && trimmed.endsWith('\''))) {
    return trimmed.substring(1, trimmed.length - 1)
  }

  return trimmed
}
