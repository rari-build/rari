/* eslint-disable no-use-before-define, no-var, vars-on-top */
if (!globalThis.React || typeof globalThis.React.createElement !== 'function') {
  globalThis.React = {
    createElement(type, props, ...children) {
      const propsWithoutKey = props ? { ...props } : {}
      const key = props && Object.hasOwn(props, 'key') ? props.key : null
      delete propsWithoutKey.key

      const element = {
        $$typeof: Symbol.for('react.transitional.element'),
        type,
        props: propsWithoutKey,
        key,
      }
      if (children.length > 0)
        element.props = { ...element.props, children: children.length === 1 ? children[0] : children }

      return element
    },
    Fragment: Symbol.for('react.fragment'),
    Suspense: Symbol.for('react.suspense'),
  }
}

function createJsxDelegate(runtimeJsx, globalJsx) {
  return (type, props, key) => {
    if (globalThis.React && typeof globalThis.React.createElement === 'function')
      return globalThis.React.createElement(type, key !== undefined ? { ...props, key } : props)
    if (typeof runtimeJsx === 'function')
      return runtimeJsx(type, props, key)
    if (typeof globalJsx === 'function')
      return globalJsx(type, props, key)

    return null
  }
}

if (typeof _jsx === 'undefined')
  var _jsx = createJsxDelegate(globalThis['~react']?.jsxRuntime?.jsx, globalThis.jsx)
if (typeof _jsxs === 'undefined')
  var _jsxs = createJsxDelegate(globalThis['~react']?.jsxRuntime?.jsxs, globalThis.jsxs)

if (typeof globalThis.jsx === 'undefined')
  globalThis.jsx = createJsxDelegate(globalThis['~react']?.jsxRuntime?.jsx, undefined)

if (typeof globalThis.jsxs === 'undefined')
  globalThis.jsxs = createJsxDelegate(globalThis['~react']?.jsxRuntime?.jsxs, undefined)

if (typeof globalThis.LoadingSpinner === 'undefined') {
  if (
    typeof document !== 'undefined'
    && typeof document.getElementById === 'function'
    && !document.getElementById('spinner-keyframes')
  ) {
    const head = document.head || document.getElementsByTagName('head')[0]
    if (head) {
      const style = document.createElement('style')
      style.id = 'spinner-keyframes'
      style.textContent = '@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }'
      head.appendChild(style)
    }
  }

  globalThis.LoadingSpinner = function () {
    if (!globalThis.React || typeof globalThis.React.createElement !== 'function')
      return null

    return globalThis.React.createElement('div', {
      style: {
        width: '40px',
        height: '40px',
        border: '4px solid #f3f4f6',
        borderTop: '4px solid #3b82f6',
        borderRadius: '50%',
        animation: 'spin 1s linear infinite',
      },
    })
  }
}

if (typeof globalThis.DefaultLoading === 'undefined') {
  globalThis.DefaultLoading = function () {
    if (!globalThis.React || typeof globalThis.React.createElement !== 'function' || !globalThis.LoadingSpinner)
      return null

    return globalThis.React.createElement('div', {
      style: {
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        padding: '2rem',
        minHeight: '200px',
      },
    }, globalThis.React.createElement(globalThis.LoadingSpinner))
  }
}

if (!globalThis.readFileSync && globalThis['~node']?.modules && globalThis['~node'].modules.get) {
  const nodeFs = globalThis['~node'].modules.get('node:fs')
  if (nodeFs && nodeFs.readFileSync) {
    globalThis.readFileSync = nodeFs.readFileSync
    globalThis.existsSync = nodeFs.existsSync
  }
  const nodePath = globalThis['~node'].modules.get('node:path')
  if (nodePath && nodePath.join)
    globalThis.join = nodePath.join
  const nodeProcess = globalThis['~node'].modules.get('node:process')
  if (nodeProcess && nodeProcess.cwd)
    globalThis.cwd = nodeProcess.cwd
}
