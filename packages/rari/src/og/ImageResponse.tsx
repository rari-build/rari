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

    const type = typeof element.type === 'string'
      ? element.type
      : element.type.name || 'div'

    const props = element.props || {}
    const children = this.serializeChildren(props.children)

    return {
      type: 'element',
      elementType: type,
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
