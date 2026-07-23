const MDX_COMPONENT_USAGE_REGEX = /<([A-Z][A-Za-z0-9]*)(?=[\s>/])/

export function scanMdxComponentNames(content: string): string[] {
  const names = new Set<string>()
  const pattern = new RegExp(MDX_COMPONENT_USAGE_REGEX.source, 'g')

  for (const match of content.matchAll(pattern)) names.add(match[1])

  return [...names]
}

export function isComponentUsedInMdx(content: string, name: string): boolean {
  return new RegExp(`<${name}[\\s>/]`).test(content)
}
