import type { ReactElement } from 'react'

export interface ImageResponseOptions {
  width?: number
  height?: number
}

export interface ImageResponseSize {
  width: number
  height: number
}

export class ImageResponse {
  private element: ReactElement
  private options: ImageResponseOptions

  constructor(element: ReactElement, options: ImageResponseOptions = {}) {
    this.element = element
    this.options = {
      width: options.width || 1200,
      height: options.height || 630,
    }
  }

  toJSON() {
    return {
      type: 'ImageResponse',
      element: this.serializeElement(this.element),
      options: this.options,
    }
  }

  private serializeElement(element: any): any {
    if (typeof element === 'string' || typeof element === 'number')
      return { type: 'text', value: String(element) }

    if (!element || !element.type)
      return null

    if (typeof element.type === 'function') {
      try {
        const rendered = element.type(element.props || {})
        if (rendered && typeof (rendered as any).then === 'function') {
          console.warn(
            `[ImageResponse] async/server component "${element.type?.name || element.type}" is not supported; skipping`,
          )
          return null
        }

        return this.serializeElement(rendered)
      }
      catch (err) {
        console.error(
          `[ImageResponse] failed to render component "${element.type?.name || element.type?.toString()}":`,
          err,
        )
        return null
      }
    }

    let resolvedType = element.type
    if (resolvedType && typeof resolvedType === 'object') {
      const REACT_MEMO = Symbol.for('react.memo')
      const REACT_FORWARD_REF = Symbol.for('react.forward_ref')
      if ((resolvedType as any).$$typeof === REACT_MEMO)
        resolvedType = (resolvedType as any).type
      else if ((resolvedType as any).$$typeof === REACT_FORWARD_REF)
        resolvedType = (resolvedType as any).render
    }

    if (typeof resolvedType === 'function') {
      try {
        const rendered = resolvedType(element.props || {})
        if (rendered && typeof (rendered as any).then === 'function') {
          console.warn(
            `[ImageResponse] async/server component "${resolvedType?.name || resolvedType}" is not supported; skipping`,
          )
          return null
        }

        return this.serializeElement(rendered)
      }
      catch (err) {
        console.error(
          `[ImageResponse] failed to render component "${resolvedType?.name || resolvedType?.toString()}":`,
          err,
        )
        return null
      }
    }

    const props = element.props || {}
    const children = this.serializeChildren(props.children)

    return {
      type: 'element',
      elementType: resolvedType,
      props: this.serializeProps(props),
      children,
    }
  }

  private serializeChildren(children: any): any[] {
    if (!children)
      return []

    if (Array.isArray(children))
      return children.map(child => this.serializeElement(child)).filter(Boolean)

    const serialized = this.serializeElement(children)
    return serialized ? [serialized] : []
  }

  private serializeProps(props: any): any {
    const { children, ...rest } = props
    const serialized: any = {}

    for (const [key, value] of Object.entries(rest)) {
      if (value !== undefined && value !== null)
        serialized[key] = value
    }

    return serialized
  }
}
