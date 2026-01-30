// oxlint-disable no-var, vars-on-top, block-scoped-var, no-unused-vars
if (typeof globalThis['~registryProxy'] === 'undefined') {
  globalThis['~registryProxy'] = new Proxy({}, {
    get(target, prop) {
      if (globalThis['~rsc'].functions && typeof globalThis['~rsc'].functions[prop] === 'function')
        return globalThis['~rsc'].functions[prop]
      if (typeof globalThis[prop] === 'function')
        return globalThis[prop]

      return undefined
    },
  })
}
