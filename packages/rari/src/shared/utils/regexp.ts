const REGEXP_SPECIAL_CHARS = /[.*+?^${}()|[\]\\]/g
const REGEXP_SPECIAL_CHARS_WITHOUT_ASTERISK = /[.+?^${}()|[\]\\]/g

export function escapeRegExp(
  value: string,
  options: { readonly escapeAsterisk?: boolean } = {},
): string {
  const { escapeAsterisk = true } = options
  const pattern = escapeAsterisk ? REGEXP_SPECIAL_CHARS : REGEXP_SPECIAL_CHARS_WITHOUT_ASTERISK
  return value.replace(pattern, '\\$&')
}
