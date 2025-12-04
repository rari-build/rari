function renderElement(element, rendered, rowId) {
  if (typeof element === 'string') {
    return escapeHtml(element)
  }

  if (typeof element === 'number' || typeof element === 'boolean') {
    return escapeHtml(String(element))
  }

  if (element === null || element === undefined) {
    return ''
  }

  if (Array.isArray(element) && element.length >= 4 && element[0] === '$') {
    const tag = element[1]
    const props = element[3] || {}
    return renderTag(tag, props, rendered, rowId)
  }

  if (element.Component) {
    const { tag, props } = element.Component
    return renderTag(tag, props || {}, rendered, rowId)
  }

  if (element.Text !== undefined) {
    return escapeHtml(String(element.Text))
  }

  if (element.Reference) {
    const ref = element.Reference
    const match = ref.match(/\$[@L]?(\d+)/)
    if (match) {
      const refId = Number.parseInt(match[1], 10)
      return rendered.get(refId) || ''
    }
    return ''
  }

  return ''
}

function renderTag(tag, props, rendered, rowId) {
  if (tag === 'react.suspense') {
    return renderSuspense(props, rendered)
  }

  if (typeof tag === 'string' && tag.startsWith('$')) {
    const match = tag.match(/\$[@L]?(\d+)/)
    if (match) {
      const refId = Number.parseInt(match[1], 10)
      const referencedContent = rendered.get(refId)

      if (referencedContent !== undefined && referencedContent !== '') {
        return referencedContent
      }

      return renderClientComponentPlaceholder(tag, props, rendered, rowId)
    }

    if (tag.startsWith('$@')) {
      return renderClientComponentPlaceholder(tag, props, rendered, rowId)
    }

    return ''
  }

  const attributes = renderAttributes(props, rowId)

  const rawContentTags = ['style', 'script']
  let children
  if (rawContentTags.includes(tag)) {
    children = renderChildrenRaw(props.children, rendered)
  }
  else {
    children = renderChildren(props.children, rendered)
  }

  const selfClosingTags = ['img', 'br', 'hr', 'input', 'meta', 'link', 'area', 'base', 'col', 'embed', 'source', 'track', 'wbr']
  if (selfClosingTags.includes(tag)) {
    return `<${tag}${attributes} />`
  }

  return `<${tag}${attributes}>${children}</${tag}>`
}

function renderSuspense(props, rendered) {
  const children = props.children

  if (typeof children === 'string' && children.startsWith('$')) {
    const match = children.match(/\$[@L]?(\d+)/)
    if (match) {
      const refId = Number.parseInt(match[1], 10)
      const resolvedContent = rendered.get(refId)

      if (resolvedContent !== undefined && resolvedContent !== '') {
        return resolvedContent
      }
    }
  }

  if (children !== undefined && children !== null && typeof children !== 'string') {
    const childrenHtml = renderElement(children, rendered, undefined)
    if (childrenHtml) {
      return childrenHtml
    }
  }

  return ''
}

function renderClientComponentPlaceholder(moduleRef, props, rendered) {
  const attributes = []

  attributes.push(`data-client-component="${escapeHtml(moduleRef)}"`)

  if (props && Object.keys(props).length > 0) {
    const propsForSerialization = {}
    for (const [key, value] of Object.entries(props)) {
      if (key !== 'children') {
        propsForSerialization[key] = value
      }
    }

    if (Object.keys(propsForSerialization).length > 0) {
      const propsJson = JSON.stringify(propsForSerialization)
      attributes.push(`data-props="${escapeHtml(propsJson)}"`)
    }
  }

  const children = props && props.children ? renderChildren(props.children, rendered) : ''

  const attrString = attributes.length > 0 ? ` ${attributes.join(' ')}` : ''
  return `<div${attrString}>${children}</div>`
}

function renderAttributes(props) {
  if (!props || typeof props !== 'object') {
    return ''
  }

  const attributes = []

  for (const [key, value] of Object.entries(props)) {
    if (key === 'children' || key === 'key' || key === 'ref') {
      continue
    }

    if (value === null || value === undefined) {
      continue
    }

    if (key.startsWith('data-')) {
      const attrValue = escapeHtml(String(value))
      attributes.push(`${key}="${attrValue}"`)
      continue
    }

    let attrName = key
    if (key === 'className') {
      attrName = 'class'
    }
    else if (key === 'htmlFor') {
      attrName = 'for'
    }

    if (typeof value === 'boolean') {
      if (value) {
        attributes.push(attrName)
      }
      continue
    }

    if (key === 'style' && typeof value === 'object') {
      const styleStr = Object.entries(value)
        .map(([k, v]) => {
          const kebabKey = k.replace(/([A-Z])/g, '-$1').toLowerCase()
          return `${kebabKey}:${v}`
        })
        .join(';')
      attributes.push(`style="${escapeHtml(styleStr)}"`)
      continue
    }

    const attrValue = escapeHtml(String(value))
    attributes.push(`${attrName}="${attrValue}"`)
  }

  return attributes.length > 0 ? ` ${attributes.join(' ')}` : ''
}

function renderChildrenRaw(children) {
  if (children === null || children === undefined) {
    return ''
  }

  if (typeof children === 'string') {
    return children
  }

  if (typeof children === 'number' || typeof children === 'boolean') {
    return String(children)
  }

  if (Array.isArray(children)) {
    return children.map((child) => {
      if (typeof child === 'string') {
        return child
      }
      if (typeof child === 'number' || typeof child === 'boolean') {
        return String(child)
      }
      return ''
    }).join('')
  }

  return ''
}

function renderChildren(children, rendered) {
  if (children === null || children === undefined) {
    return ''
  }

  if (typeof children === 'string' && children.startsWith('$')) {
    const match = children.match(/\$[@L]?(\d+)/)
    if (match) {
      const refId = Number.parseInt(match[1], 10)
      return rendered.get(refId) || ''
    }
  }

  if (Array.isArray(children)) {
    if (children.length >= 4 && children[0] === '$') {
      return renderElement(children, rendered, undefined)
    }

    const renderedChildren = []
    let hasMultipleTextNodes = false
    let textNodeCount = 0

    for (const child of children) {
      if (typeof child === 'string' || typeof child === 'number') {
        textNodeCount++
      }
    }
    hasMultipleTextNodes = textNodeCount > 1

    for (let i = 0; i < children.length; i++) {
      const child = children[i]
      const isTextNode = typeof child === 'string' || typeof child === 'number'

      if (isTextNode && hasMultipleTextNodes) {
        const renderedChild = renderElement(child, rendered, undefined)
        if (renderedChild) {
          renderedChildren.push(`<!-- -->${renderedChild}<!-- -->`)
        }
      }
      else {
        renderedChildren.push(renderElement(child, rendered, undefined))
      }
    }

    return renderedChildren.join('')
  }

  return renderElement(children, rendered, undefined)
}

function escapeHtml(text) {
  if (typeof text !== 'string') {
    text = String(text)
  }

  const htmlEscapeMap = {
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    '\'': '&#39;',
  }

  return text.replace(/[&<>"']/g, char => htmlEscapeMap[char])
}

globalThis.renderRscToHtml = function (rscRows) {
  const rendered = new Map()

  let lastRowId = -1
  for (const row of rscRows) {
    lastRowId = row.id
  }

  for (const row of rscRows) {
    const html = renderElement(row.data, rendered, row.id)
    rendered.set(row.id, html)
  }

  return rendered.get(lastRowId) || rendered.get(0) || ''
}
