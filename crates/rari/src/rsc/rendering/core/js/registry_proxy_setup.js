/* eslint-disable no-use-before-define */
// oxlint-disable no-var, vars-on-top, block-scoped-var, no-unused-vars
if (typeof __registry_proxy === 'undefined') {
  var __registry_proxy = new Proxy({}, {
    get(target, prop) {
      if (globalThis['~rsc'].functions && typeof globalThis['~rsc'].functions[prop] === 'function') {
        return globalThis['~rsc'].functions[prop]
      }
      if (typeof globalThis[prop] === 'function') {
        return globalThis[prop]
      }
      return undefined
    },
  })
}
