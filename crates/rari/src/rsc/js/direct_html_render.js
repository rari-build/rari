function escapeHtml(text) {
  if (text === null || text === undefined) {
    return ''
  }

  const str = String(text)
  const map = {
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    '\'': '&#039;',
  }

  return str.replace(/[&<>"']/g, m => map[m])
}

function kebabCase(str) {
  return str.replace(/([a-z])([A-Z])/g, '$1-$2').toLowerCase()
}

function isSelfClosing(tagName) {
  const selfClosingTags = new Set([
    'area',
    'base',
    'br',
    'col',
    'embed',
    'hr',
    'img',
    'input',
    'link',
    'meta',
    'param',
    'source',
    'track',
    'wbr',
  ])
  return selfClosingTags.has(tagName.toLowerCase())
}

async function renderHtmlElement(tagName, props, depth) {
  const { children, ...attributes } = props

  let html = `<${tagName}`

  for (const [key, value] of Object.entries(attributes)) {
    if (key === 'key' || key === 'ref' || key === '__self' || key === '__source') {
      continue
    }

    if (key === 'className') {
      if (value) {
        html += ` class="${escapeHtml(value)}"`
      }
      continue
    }

    if (key === 'style' && typeof value === 'object' && value !== null) {
      const styleStr = Object.entries(value)
        .map(([k, v]) => `${kebabCase(k)}:${v}`)
        .join(';')
      if (styleStr) {
        html += ` style="${escapeHtml(styleStr)}"`
      }
      continue
    }

    if (typeof value === 'boolean') {
      if (value) {
        html += ` ${key}`
      }
      continue
    }

    if (typeof value === 'string' || typeof value === 'number') {
      html += ` ${key}="${escapeHtml(String(value))}"`
      continue
    }
  }

  if (isSelfClosing(tagName)) {
    html += ' />'
    return html
  }

  html += '>'

  if (children !== undefined && children !== null) {
    html += await renderToHtmlDirect(children, depth + 1)
  }

  html += `</${tagName}>`

  return html
}

async function renderToHtmlDirect(element, depth = 0) {
  if (depth > 100) {
    console.error('HTML render depth limit exceeded')
    return '<div style="color:red">Error: Render depth limit exceeded</div>'
  }

  if (element === null || element === undefined) {
    return ''
  }

  if (element && typeof element === 'object' && typeof element.then === 'function') {
    try {
      element = await element
      return await renderToHtmlDirect(element, depth)
    }
    catch (error) {
      console.error('Error awaiting Promise in HTML render:', error)
      return `<div style="color:red">Error: ${escapeHtml(error.message)}</div>`
    }
  }

  if (typeof element === 'string' || typeof element === 'number') {
    return escapeHtml(String(element))
  }

  if (typeof element === 'boolean') {
    return ''
  }

  if (Array.isArray(element)) {
    const results = []
    for (const child of element) {
      results.push(await renderToHtmlDirect(child, depth + 1))
    }
    return results.join('')
  }

  if (element && typeof element === 'object') {
    const type = element.type
    const props = element.props || {}

    if (typeof type === 'string') {
      return await renderHtmlElement(type, props, depth)
    }

    if (type === Symbol.for('react.fragment') || (type && type === globalThis.React?.Fragment)) {
      return await renderToHtmlDirect(props.children, depth + 1)
    }

    if (typeof type === 'function') {
      try {
        let rendered = type(props)

        if (rendered && typeof rendered.then === 'function') {
          rendered = await rendered
        }

        return await renderToHtmlDirect(rendered, depth + 1)
      }
      catch (error) {
        console.error('Error rendering function component:', error)
        return `<div style="color:red">Error: ${escapeHtml(error.message)}</div>`
      }
    }

    if (props.children !== undefined) {
      return await renderToHtmlDirect(props.children, depth + 1)
    }
  }

  return ''
}

if (typeof globalThis !== 'undefined') {
  globalThis.renderToHtmlDirect = renderToHtmlDirect
  globalThis.escapeHtml = escapeHtml
  globalThis.kebabCase = kebabCase
}
