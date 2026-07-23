import type { EvaluateOptions as MdxEvaluateOptions } from '@mdx-js/mdx'
import type { ComponentType } from 'react'
import { evaluate as evaluateMdx } from '@mdx-js/mdx'
import { getMDXComponents } from 'rari/mdx/registry'
import { createElement } from 'react'

export interface EvaluateOptions extends MdxEvaluateOptions {
  components?: Record<string, any>
}

export interface EvaluateResult {
  default: ComponentType<{ components?: Record<string, any> }>
}

export async function evaluate(source: string, options: EvaluateOptions): Promise<EvaluateResult> {
  const { components: userComponents = {}, ...evaluateOptions } = options
  const compiled = await evaluateMdx(source, evaluateOptions)
  const MDXContent = compiled.default
  const registryComponents = getMDXComponents(source)

  function RariMDXContent(
    props: Readonly<{ readonly components?: { readonly [key: string]: any } }>,
  ) {
    const mergedComponents = {
      ...registryComponents,
      ...userComponents,
      ...props.components,
    }

    return createElement(MDXContent, {
      ...props,
      components: mergedComponents,
    })
  }

  return {
    ...compiled,
    default: RariMDXContent,
  }
}
