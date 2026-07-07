import { HTML_ESCAPE_REGEXES } from '@/shared/regex-constants'

export function escapeXml(str: string): string {
  return str
    .replace(HTML_ESCAPE_REGEXES.AMPERSAND, '&amp;')
    .replace(HTML_ESCAPE_REGEXES.LT, '&lt;')
    .replace(HTML_ESCAPE_REGEXES.GT, '&gt;')
    .replace(HTML_ESCAPE_REGEXES.QUOTE, '&quot;')
    .replace(HTML_ESCAPE_REGEXES.APOS, '&apos;')
}
