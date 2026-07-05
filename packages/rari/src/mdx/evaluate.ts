import type { EvaluateOptions as MdxEvaluateOptions } from '@mdx-js/mdx'
import type { ComponentType, ElementType, ExoticComponent, FragmentProps, Key, ReactElement } from 'react'
import { evaluate as evaluateMdx } from '@mdx-js/mdx'
import { getMDXComponents } from 'rari/mdx/registry'
import { createElement } from 'react'

export interface EvaluateOptions {
  components?: Record<string, any>
  baseUrl?: string | URL
  development?: boolean
  remarkPlugins?: unknown[]
  rehypePlugins?: unknown[]
  recmaPlugins?: unknown[]
  Fragment?: ExoticComponent<FragmentProps>
  jsx?: (type: ElementType, props: unknown, key?: Key) => ReactElement
  jsxs?: (type: ElementType, props: unknown, key?: Key) => ReactElement
  jsxDEV?: (type: ElementType, props: unknown, key?: Key, isStaticChildren?: boolean, source?: object, self?: unknown) => ReactElement
  useMDXComponents?: () => Record<string, ComponentType<any>>
  jsxImportSource?: string
  format?: 'detect' | 'mdx' | 'md'
  outputFormat?: 'program' | 'function-body'
  [key: string]: unknown
}

export interface EvaluateResult {
  default: ComponentType<{ components?: Record<string, any> }>
}

export async function evaluate(source: string, options: EvaluateOptions = {}): Promise<EvaluateResult> {
  const { components: userComponents = {}, ...evaluateOptions } = options
  const compiled = await evaluateMdx(source, evaluateOptions as MdxEvaluateOptions)
  const MDXContent = compiled.default

  function RariMDXContent(props: { components?: Record<string, any> }) {
    const mergedComponents = {
      ...getMDXComponents(source),
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
