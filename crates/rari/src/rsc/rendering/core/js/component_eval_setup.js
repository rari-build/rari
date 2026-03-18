/* eslint-disable no-use-before-define, no-var, vars-on-top */
if (typeof _jsx === 'undefined')
  var _jsx = globalThis['~react']?.jsxRuntime?.jsx || (() => null)
if (typeof _jsxs === 'undefined')
  var _jsxs = globalThis['~react']?.jsxRuntime?.jsxs || (() => null)

if (typeof globalThis.jsx === 'undefined') {
  globalThis.jsx = function (type, props, key) {
    if (!globalThis.React)
      return null

    return globalThis.React.createElement(type, { ...props, key })
  }
}

if (typeof globalThis.jsxs === 'undefined') {
  globalThis.jsxs = function (type, props, key) {
    if (!globalThis.React)
      return null

    return globalThis.React.createElement(type, { ...props, key })
  }
}

if (typeof globalThis.LoadingSpinner === 'undefined') {
  if (typeof document !== 'undefined' && !document.getElementById('spinner-keyframes')) {
    const style = document.createElement('style')
    style.id = 'spinner-keyframes'
    style.textContent = '@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }'
    document.head.appendChild(style)
  }

  globalThis.LoadingSpinner = function () {
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
