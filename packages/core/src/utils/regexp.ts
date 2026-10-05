const ESCAPED_ASTERISK = /\\\*/g

export function escapeRegExp(
  value: string,
  options: { readonly escapeAsterisk?: boolean } = {},
): string {
  const { escapeAsterisk = true } = options
  const escaped = RegExp.escape(value)
  return escapeAsterisk ? escaped : escaped.replace(ESCAPED_ASTERISK, '*')
}
