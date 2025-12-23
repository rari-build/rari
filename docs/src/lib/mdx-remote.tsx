import type { CompileOptions } from '@mdx-js/mdx'
import { compile } from '@mdx-js/mdx'

interface MDXRemoteProps {
  source: string
  options?: CompileOptions
}

export async function compileMDXRemote({ source, options = {} }: MDXRemoteProps) {
  const compiled = await compile(source, {
    outputFormat: 'function-body',
    development: false,
    ...options,
  })

  return String(compiled)
}
