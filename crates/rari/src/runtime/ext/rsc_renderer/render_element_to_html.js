if (typeof globalThis.renderElementToHtml === 'undefined') {
  const safeStringify = (obj, maxDepth = 3) => {
    const seen = new WeakSet()

    const stringify = (value, depth = 0) => {
      if (depth > maxDepth)
        return '"[Max Depth Reached]"'

      if (value === null)
        return 'null'
      if (value === undefined)
        return 'undefined'
      if (typeof value === 'string')
        return JSON.stringify(value)
      if (typeof value === 'number' || typeof value === 'boolean')
        return String(value)
      if (typeof value === 'function')
        return `"[Function: ${value.name || 'anonymous'}]"`
      if (typeof value === 'symbol')
        return `"[Symbol: ${value.toString()}]"`

      if (typeof value === 'object') {
        if (seen.has(value))
          return '"[Circular Reference]"'
        seen.add(value)

        if (Array.isArray(value)) {
          const items = value
            .slice(0, 10)
            .map(item => stringify(item, depth + 1))
          const result = `[${items.join(', ')}${value.length > 10 ? ', ...' : ''}]`
          seen.delete(value)
          return result
        }

        if (value.constructor && value.constructor.name) {
          const constructorName = value.constructor.name
          if (
            constructorName === 'DedicatedWorkerGlobalScope'
            || constructorName === 'Window'
          ) {
            seen.delete(value)
            return `"[${constructorName}]"`
          }
        }

        const entries = Object.entries(value)
          .slice(0, 5)
          .map(([k, v]) => `${JSON.stringify(k)}: ${stringify(v, depth + 1)}`)
        const result = `{${entries.join(', ')}${Object.keys(value).length > 5 ? ', ...' : ''}}`
        seen.delete(value)
        return result
      }

      return '"[Unknown Type]"'
    }

    try {
      return stringify(obj)
    }
    catch (error) {
      return `"[Stringify Error: ${error.message}]"`
    }
  }

  globalThis.renderElementToHtml = function (element) {
    try {
      if (
        globalThis.ReactDOMServer
        && typeof globalThis.ReactDOMServer.renderToString === 'function'
      ) {
        try {
          const result = globalThis.ReactDOMServer.renderToString(element)
          return result
        }
        catch (reactError) {
          if (
            reactError
            && reactError.$$typeof === Symbol.for('react.suspense.pending')
          ) {
            throw reactError
          }
          console.warn('renderElementToHtml: ReactDOMServer failed, falling back to custom renderer:', reactError.message)
        }
      }

      if (!element) {
        return ''
      }

      if (typeof element !== 'object') {
        return String(element)
      }

      if (element.$$typeof && element.type) {
        const type = element.type
        const props = element.props || {}
        const children = props.children || []

        if (typeof type === 'string') {
          let html = `<${type}`

          for (const [key, value] of Object.entries(props)) {
            if (key !== 'children' && value !== undefined) {
              const attr = key === 'className' ? 'class' : key
              html += ` ${attr}="${String(value)}"`
            }
          }

          const selfClosingTags = ['img', 'input', 'br', 'hr', 'meta', 'link']
          if (selfClosingTags.includes(type)) {
            return `${html}/>`
          }

          html += '>'

          if (Array.isArray(children)) {
            let lastChildWasString = false
            let lastWasElement = false

            for (const child of children) {
              const childType = typeof child
              const isString = childType === 'string'
              const isElement = child !== null && typeof child === 'object'

              if (lastChildWasString && isString && child.trim() === '') {
                html += '\u00A0'
              }

              if (lastChildWasString && isElement) {
                html += ' '
              }

              if (lastWasElement && isString && child.startsWith(' ')) {
                html += ' '
              }

              html += globalThis.renderElementToHtml(child)

              lastChildWasString = isString
              lastWasElement = isElement
            }
          }
          else if (children !== null && children !== undefined) {
            html += globalThis.renderElementToHtml(children)
          }

          html += `</${type}>`
          return html
        }
        else if (type === Symbol.for('react.fragment')) {
          let html = ''

          if (Array.isArray(children)) {
            let lastChildWasString = false
            let lastWasElement = false

            for (const child of children) {
              const childType = typeof child
              const isString = childType === 'string'
              const isElement = child !== null && typeof child === 'object'

              if (lastChildWasString && isString && child.trim() === '') {
                html += '\u00A0'
              }

              if (lastChildWasString && isElement) {
                html += ' '
              }

              if (lastWasElement && isString && child.startsWith(' ')) {
                html += ' '
              }

              html += globalThis.renderElementToHtml(child)

              lastChildWasString = isString
              lastWasElement = isElement
            }
          }
          else if (children !== null && children !== undefined) {
            html += globalThis.renderElementToHtml(children)
          }

          return html
        }
        else if (typeof type === 'function') {
          try {
            if (globalThis.__track_component_render && type.name) {
              globalThis.__track_component_render(type.name)
            }

            const result = type(props)

            if (result && typeof result.then === 'function') {
              return result
            }

            return globalThis.renderElementToHtml(result)
          }
          catch (error) {
            if (
              error
              && error.$$typeof === Symbol.for('react.suspense.pending')
            ) {
              throw error
            }

            return `<div class="error">Error rendering component: ${error.message}</div>`
          }
        }

        return `<div>Unknown element type: ${typeof type}</div>`
      }

      if (Array.isArray(element)) {
        let html = ''
        let lastWasString = false
        let lastWasElement = false

        for (let i = 0; i < element.length; i++) {
          const item = element[i]
          const isString = typeof item === 'string' || typeof item === 'number'
          const isElement = item !== null && typeof item === 'object'

          if (lastWasString && isString) {
            const stringValue = String(item)
            if (stringValue.startsWith(' ') || stringValue.endsWith(' ')) {
              html += globalThis.renderElementToHtml(item)
            }
            else {
              html += ` ${globalThis.renderElementToHtml(item)}`
            }
          }
          else if (lastWasString && isElement) {
            html += ` ${globalThis.renderElementToHtml(item)}`
          }
          else if (lastWasElement && isString) {
            const stringValue = String(item)
            if (stringValue.startsWith(' ')) {
              html += ` ${globalThis.renderElementToHtml(item)}`
            }
            else {
              html += globalThis.renderElementToHtml(item)
            }
          }
          else {
            html += globalThis.renderElementToHtml(item)
          }

          lastWasString = isString
          lastWasElement = isElement
        }

        return html
      }

      return `<pre>${safeStringify(element)}</pre>`
    }
    catch (error) {
      if (error && error.$$typeof === Symbol.for('react.suspense.pending')) {
        throw error
      }

      return `<div class="render-error">Rendering error: ${error.message}</div>`
    }
  }
}
