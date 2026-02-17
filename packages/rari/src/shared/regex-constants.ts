export const TSX_EXT_REGEX = /\.(?:tsx?|jsx?)$/
export const FILE_EXTENSION_REGEX = /\.[jt]sx?$/
export const JS_EXTENSION_REGEX = /\.js$/

export const WINDOWS_PATH_REGEX = /^[A-Z]:\\/i
export const BACKSLASH_REGEX = /\\/g
export const PATH_SEPARATOR_REGEX = /[/\\]/
export const PATH_SLASHES_REGEX = /\//g
export const PATH_TRAILING_SLASH_REGEX = /\/$/g
export const PATH_START_SLASH_REGEX = /^\//
export const SRC_PREFIX_REGEX = /^src\//
export const FILE_PROTOCOL_REGEX = /^file:\/\//

export const HTML_ESCAPE_REGEXES = {
  AMPERSAND: /&/g,
  LT: /</g,
  GT: />/g,
  QUOTE: /"/g,
  SINGLE_QUOTE: /'/g,
  APOS: /'/g,
} as const

export const HTML_AMPERSAND_REGEX = HTML_ESCAPE_REGEXES.AMPERSAND
export const HTML_LT_REGEX = HTML_ESCAPE_REGEXES.LT
export const HTML_GT_REGEX = HTML_ESCAPE_REGEXES.GT
export const HTML_QUOTE_REGEX = HTML_ESCAPE_REGEXES.QUOTE
export const HTML_SINGLE_QUOTE_REGEX = HTML_ESCAPE_REGEXES.SINGLE_QUOTE
export const HTML_APOS_REGEX = HTML_ESCAPE_REGEXES.APOS

export const XML_AMPERSAND_REGEX = /&/g
export const XML_LT_REGEX = /</g
export const XML_GT_REGEX = />/g
export const XML_QUOTE_REGEX = /"/g
export const XML_APOS_REGEX = /'/g

export const NON_ALPHANUMERIC_REGEX = /[^a-z0-9]/gi
export const COMPONENT_ID_REGEX = /[^\w/-]/g
export const NUMERIC_REGEX = /^\d+$/

export const CAMEL_CASE_REGEX = /([a-z])([A-Z])/g
export const CAMEL_TO_KEBAB_REGEX = /([A-Z])/g

export const EXPORT_DEFAULT_REGEX = /export\s+default\s+/
export const EXPORT_DEFAULT_FUNCTION_REGEX = /^export\s+default\s+function\s+(\w+)/gm
export const EXPORT_DEFAULT_ASYNC_FUNCTION_REGEX = /^export\s+default\s+async\s+function\s+(\w+)/gm
export const EXPORT_DEFAULT_NAME_REGEX = /^export\s+default\s+(\w+);?\s*$/gm
export const EXPORT_DEFAULT_AS_REGEX = /^export\s*\{\s*(\w+)\s+as\s+default\s*\};?\s*$/gm
export const EXPORT_DEFAULT_TEST_REGEX = /export\s+default\s+/
export const EXPORT_FUNCTION_REGEX = /export\s+(async\s+)?function\s+(\w+)/g
export const EXPORT_NAMED_DECLARATION_REGEX = /export\s+(?:function|const|class)\s+(\w+)/

export const TITLE_EXPORT_REGEX = /^export\s+const\s+title\s*=\s*['"](.+)['"]/m
export const DESCRIPTION_EXPORT_REGEX = /^export\s+const\s+description\s*=\s*['"](.+)['"]/m
export const DATE_EXPORT_REGEX = /^export\s+const\s+date\s*=\s*['"](.+)['"]/m
export const AUTHOR_EXPORT_REGEX = /^export\s+const\s+author\s*=\s*['"](.+)['"]/m

export const EXTENSION_REGEX = /\.[^.]*$/
export const NEWLINE_REGEX = /\r?\n/
export const WHITESPACE_REGEX = /\s+/g
export const MULTIPLE_DASHES_REGEX = /-{2,}/g
export const NON_WORD_REGEX = /[^\w-]+/g
export const QUOTE_REGEX = /['"]/g
export const HEX_REGEX = /^#[0-9A-F]{6}$/i
