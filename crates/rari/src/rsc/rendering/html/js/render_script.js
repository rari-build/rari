function renderElement(element, rendered, rowId, moduleMap) {
  if (typeof element === 'string')
    return escapeHtml(element)

  if (typeof element === 'number' || typeof element === 'boolean')
    return escapeHtml(String(element))

  if (element === null || element === undefined)
    return ''

  if (Array.isArray(element) && element.length >= 4 && element[0] === '$') {
    const tag = element[1]
    const props = element[3] || {}
    return renderTag(tag, props, rendered, rowId, moduleMap)
  }

  if (element.Component) {
    const { tag, props } = element.Component
    return renderTag(tag, props || {}, rendered, rowId, moduleMap)
  }

  if (element.Text !== undefined)
    return escapeHtml(String(element.Text))

  if (element.Reference) {
    const ref = element.Reference

    if (ref.startsWith('$S'))
      return ''

    const match = ref.match(/\$[@L]?(\d+)/)
    if (match) {
      const refId = Number.parseInt(match[1], 10)
      return rendered.get(refId) || ''
    }

    return ''
  }

  if (element.Suspense) {
    const { fallback_ref, children_ref, boundary_id } = element.Suspense

    let fallback = null
    if (fallback_ref) {
      const match = fallback_ref.match(/\$[@L]?(\d+)/)
      if (match) {
        const refId = Number.parseInt(match[1], 10)
        fallback = rendered.get(refId)
      }
      else {
        try {
          const fallbackData = JSON.parse(fallback_ref)
          fallback = renderElement(fallbackData, rendered, undefined, moduleMap)
        }
        catch {
          fallback = fallback_ref
        }
      }
    }

    let children = null
    if (children_ref) {
      const match = children_ref.match(/\$[@L]?(\d+)/)
      if (match) {
        const refId = Number.parseInt(match[1], 10)
        children = rendered.get(refId)
      }
      else {
        try {
          const childrenData = JSON.parse(children_ref)
          if (childrenData && typeof childrenData === 'object') {
            const isLazy = childrenData.__rari_lazy === true
              || (Array.isArray(childrenData) && childrenData.length > 0 && childrenData[0].__rari_lazy === true)

            if (!isLazy)
              children = renderElement(childrenData, rendered, undefined, moduleMap)
          }
          else {
            children = renderElement(childrenData, rendered, undefined, moduleMap)
          }
        }
        catch {
          if (children_ref !== 'null' && children_ref !== '')
            children = children_ref
        }
      }
    }

    if (children !== null && children !== undefined && children !== '')
      return children

    if (fallback !== null && fallback !== undefined && fallback !== '') {
      if (boundary_id)
        return `<div data-boundary-id="${escapeHtml(boundary_id)}" class="rari-suspense-boundary">${fallback}</div>`

      return fallback
    }

    return ''
  }

  return ''
}

function renderTag(tag, props, rendered, rowId, moduleMap) {
  if (tag === 'react.suspense' || tag === '$0')
    return renderSuspense(props, rendered, moduleMap)

  if (typeof tag === 'string' && tag.startsWith('$')) {
    const match = tag.match(/\$[@L]?(\d+)/)
    if (match) {
      const refId = Number.parseInt(match[1], 10)
      const referencedContent = rendered.get(refId)

      if (referencedContent !== undefined && referencedContent !== '')
        return referencedContent

      return renderClientComponentPlaceholder(tag, props, rendered, rowId, moduleMap)
    }

    if (tag.startsWith('$@'))
      return renderClientComponentPlaceholder(tag, props, rendered, rowId, moduleMap)

    return ''
  }

  const attributes = renderAttributes(props)

  if (props.dangerouslySetInnerHTML && typeof props.dangerouslySetInnerHTML === 'object' && '__html' in props.dangerouslySetInnerHTML) {
    const selfClosingTags = ['img', 'br', 'hr', 'input', 'meta', 'link', 'area', 'base', 'col', 'embed', 'source', 'track', 'wbr']
    if (selfClosingTags.includes(tag))
      return `<${tag}${attributes} />`

    return `<${tag}${attributes}>${props.dangerouslySetInnerHTML.__html}</${tag}>`
  }

  const rawContentTags = ['style', 'script']
  let children
  if (rawContentTags.includes(tag))
    children = renderChildrenRaw(props.children)
  else
    children = renderChildren(props.children, rendered, moduleMap)

  const selfClosingTags = ['img', 'br', 'hr', 'input', 'meta', 'link', 'area', 'base', 'col', 'embed', 'source', 'track', 'wbr']
  if (selfClosingTags.includes(tag))
    return `<${tag}${attributes} />`

  return `<${tag}${attributes}>${children}</${tag}>`
}

function renderSuspense(props, rendered, moduleMap) {
  const children = props.children
  const fallback = props.fallback
  const boundaryId = props['~boundaryId']

  if (typeof children === 'string' && children.startsWith('$')) {
    const match = children.match(/\$[@L]?(\d+)/)
    if (match) {
      const refId = Number.parseInt(match[1], 10)
      const resolvedContent = rendered.get(refId)

      if (resolvedContent !== undefined && resolvedContent !== '')
        return resolvedContent
    }
  }

  if (children !== undefined && children !== null && typeof children !== 'string') {
    const childrenHtml = renderElement(children, rendered, undefined, moduleMap)
    if (childrenHtml)
      return childrenHtml
  }

  if (children === null || children === undefined || children === '') {
    if (fallback !== undefined && fallback !== null) {
      const fallbackHtml = renderElement(fallback, rendered, undefined, moduleMap)

      if (boundaryId)
        return `<div data-boundary-id="${escapeHtml(boundaryId)}" class="rari-suspense-boundary">${fallbackHtml}</div>`

      return fallbackHtml
    }
  }

  return ''
}

function renderClientComponentPlaceholder(moduleRef, props, rendered, rowId, moduleMap) {
  const attributes = []

  const componentPath = moduleMap && moduleMap.get(moduleRef)
  attributes.push(`data-client-component="${escapeHtml(componentPath || moduleRef)}"`)

  if (props && Object.keys(props).length > 0) {
    const propsForSerialization = {}
    for (const [key, value] of Object.entries(props)) {
      if (key !== 'children')
        propsForSerialization[key] = value
    }

    if (Object.keys(propsForSerialization).length > 0) {
      const propsJson = JSON.stringify(propsForSerialization)
      attributes.push(`data-props="${escapeHtml(propsJson)}"`)
    }
  }

  attributes.push('style="display: contents;"')

  const children = props && props.children ? renderChildren(props.children, rendered, moduleMap) : ''

  const attrString = attributes.length > 0 ? ` ${attributes.join(' ')}` : ''
  return `<div${attrString}>${children}</div>`
}

function renderAttributes(props) {
  if (!props || typeof props !== 'object')
    return ''

  const attributes = []

  for (const [key, value] of Object.entries(props)) {
    if (key === 'children' || key === 'key' || key === 'ref' || key === 'dangerouslySetInnerHTML')
      continue

    if (value === null || value === undefined)
      continue

    if (key.startsWith('data-')) {
      const attrValue = escapeHtml(String(value))
      attributes.push(`${key}="${attrValue}"`)
      continue
    }

    let attrName = key
    if (key === 'className')
      attrName = 'class'
    else if (key === 'htmlFor')
      attrName = 'for'

    if (typeof value === 'boolean') {
      if (value)
        attributes.push(attrName)
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
  if (children === null || children === undefined)
    return ''

  if (typeof children === 'string')
    return children

  if (typeof children === 'number' || typeof children === 'boolean')
    return String(children)

  if (Array.isArray(children)) {
    return children.map((child) => {
      if (typeof child === 'string')
        return child
      if (typeof child === 'number' || typeof child === 'boolean')
        return String(child)

      return ''
    }).join('')
  }

  return ''
}

function renderChildren(children, rendered, moduleMap) {
  if (children === null || children === undefined)
    return ''

  if (typeof children === 'string' && children.startsWith('$')) {
    const match = children.match(/\$[@L]?(\d+)/)
    if (match) {
      const refId = Number.parseInt(match[1], 10)
      return rendered.get(refId) || ''
    }
  }

  if (Array.isArray(children)) {
    if (children.length >= 4 && children[0] === '$')
      return renderElement(children, rendered, undefined, moduleMap)

    const renderedChildren = []
    let hasMultipleTextNodes = false
    let textNodeCount = 0

    for (const child of children) {
      if (typeof child === 'string' || typeof child === 'number')
        textNodeCount++
    }
    hasMultipleTextNodes = textNodeCount > 1

    for (let i = 0; i < children.length; i++) {
      const child = children[i]
      const isTextNode = typeof child === 'string' || typeof child === 'number'

      if (isTextNode && hasMultipleTextNodes) {
        const renderedChild = renderElement(child, rendered, undefined, moduleMap)
        if (renderedChild)
          renderedChildren.push(`<!-- -->${renderedChild}<!-- -->`)
      }
      else {
        renderedChildren.push(renderElement(child, rendered, undefined, moduleMap))
      }
    }

    return renderedChildren.join('')
  }

  return renderElement(children, rendered, undefined, moduleMap)
}

function escapeHtml(text) {
  if (typeof text !== 'string')
    text = String(text)

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
  const moduleMap = new Map()

  let lastRowId = -1
  for (const row of rscRows)
    lastRowId = row.id

  for (const row of rscRows) {
    if (row.data && row.data.Text) {
      const text = row.data.Text
      if (text.startsWith('I[')) {
        try {
          const importData = JSON.parse(text.substring(1))
          if (Array.isArray(importData) && importData.length >= 3) {
            const [filePath, , exportName] = importData
            const componentPath = `${filePath}#${exportName}`
            const moduleRef = `$L${row.id}`
            moduleMap.set(moduleRef, componentPath)
            rendered.set(row.id, '')
          }
        }
        catch {}
      }
    }
  }

  for (const row of rscRows) {
    if (rendered.has(row.id))
      continue

    const html = renderElement(row.data, rendered, row.id, moduleMap)
    rendered.set(row.id, html)
  }

  return rendered.get(lastRowId) || rendered.get(0) || ''
}
