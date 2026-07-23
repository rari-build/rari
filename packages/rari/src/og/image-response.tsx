import type { ReactElement } from 'react'
import { isReactElementLike, isRecord } from '@/shared/utils/type-guards'

export interface ImageResponseOptions {
  readonly width?: number
  readonly height?: number
}

export interface ImageResponseSize {
  readonly width: number
  readonly height: number
}

const REACT_MEMO = Symbol.for('react.memo')
const REACT_FORWARD_REF = Symbol.for('react.forward_ref')

interface SerializedTextElement {
  type: 'text'
  value: string
}

interface SerializedTreeElement {
  type: 'element'
  elementType: unknown
  props: Record<string, unknown>
  children: SerializedElement[]
}

type SerializedElement = SerializedTextElement | SerializedTreeElement

interface ComponentLike {
  $$typeof?: symbol
  type?: unknown
  render?: unknown
  name?: string
  toString: () => string
}

type RenderableComponent = ComponentLike | ((props: Readonly<Record<string, unknown>>) => unknown)

function isComponentLike(value: unknown): value is ComponentLike {
  return isRecord(value) && typeof value.toString === 'function'
}

function isThenable(value: unknown): value is { then: unknown } {
  return isRecord(value) && 'then' in value
}

function unwrapComponentType(type: unknown): RenderableComponent | null {
  let resolved: unknown = type

  while (isComponentLike(resolved)) {
    if (resolved.$$typeof === REACT_MEMO && resolved.type !== undefined) resolved = resolved.type
    else if (resolved.$$typeof === REACT_FORWARD_REF && resolved.render !== undefined)
      resolved = resolved.render
    else break
  }

  if (typeof resolved === 'function' || isComponentLike(resolved)) return resolved

  return null
}

export class ImageResponse {
  private readonly element: ReactElement
  private readonly options: ImageResponseOptions

  constructor(element: ReactElement, options: ImageResponseOptions = {}) {
    this.element = element
    this.options = {
      width: options.width != null && options.width !== 0 ? options.width : 1200,
      height: options.height != null && options.height !== 0 ? options.height : 630,
    }
  }

  toJSON() {
    return {
      type: 'ImageResponse',
      element: this.serializeElement(this.element),
      options: this.options,
    }
  }

  private resolveAndInvoke(
    type: unknown,
    props: Readonly<Record<string, unknown>>,
  ): SerializedElement | null {
    const resolved = unwrapComponentType(type)
    if (resolved == null || typeof resolved !== 'function') return null

    try {
      const rendered = resolved(props)
      if (rendered !== null && isThenable(rendered)) {
        console.warn(
          `[ImageResponse] async/server component "${resolved.name || resolved.toString()}" is not supported; skipping`,
        )
        return null
      }

      return this.serializeElement(rendered)
    } catch (err) {
      console.error(
        `[ImageResponse] failed to render component "${resolved.name || resolved.toString()}":`,
        err,
      )
      return null
    }
  }

  private serializeElement(element: unknown): SerializedElement | null {
    if (typeof element === 'string' || typeof element === 'number')
      return { type: 'text', value: String(element) }

    if (!isRecord(element) || !isReactElementLike(element)) return null

    const { type, props = {} } = element

    const unwrapped = unwrapComponentType(type)
    if (typeof unwrapped === 'function' || (unwrapped != null && isComponentLike(unwrapped)))
      return this.resolveAndInvoke(type, props)

    const children = this.serializeChildren(props.children)

    return {
      type: 'element',
      elementType: type,
      props: this.serializeProps(props),
      children,
    }
  }

  private serializeChildren(children: unknown): SerializedElement[] {
    if (children == null) return []

    if (Array.isArray(children)) {
      return children.flatMap(child => {
        const serialized = this.serializeElement(child)
        return serialized ? [serialized] : []
      })
    }

    const serialized = this.serializeElement(children)
    return serialized ? [serialized] : []
  }

  private serializeProps(props: Readonly<Record<string, unknown>>): Record<string, unknown> {
    const result: Record<string, unknown> = {}

    for (const [key, value] of Object.entries(props)) {
      if (key === 'children') continue

      if (
        value == null ||
        typeof value === 'string' ||
        typeof value === 'number' ||
        typeof value === 'boolean'
      )
        result[key] = value
    }

    return result
  }
}
