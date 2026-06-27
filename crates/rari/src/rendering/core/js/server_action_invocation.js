/* eslint-disable no-undef, style/object-curly-spacing */
// oxlint-disable @typescript-eslint/no-floating-promises, @typescript-eslint/unbound-method
(async () => {
  try {
    if (!globalThis.getServerFunction) {
      throw new TypeError('Server function registry not initialized (~serverFunctions from init_rsc_modules.js has not run)')
    }

    const fn = globalThis.getServerFunction('{function_name}')

    if (!fn)
      throw new TypeError('Server function \'{function_name}\' not found or not registered')

    if (typeof fn !== 'function')
      throw new TypeError('Server function \'{function_name}\' is not a function')

    const hasOwn = Object.prototype.hasOwnProperty
    const rawArgs = {args_json}
    const processedArgs = rawArgs.map((arg) => {
      if (arg && typeof arg === 'object' && !Array.isArray(arg) && !(arg instanceof FormData)) {
        const formDataLike = {
          data: arg,
          get(key) {
            if (hasOwn.call(this.data, key))
              return this.data[key]

            return undefined
          },
          has(key) {
            return hasOwn.call(this.data, key)
          },
          set(key, value) {
            if (key === '__proto__' || key === 'constructor' || key === 'prototype')
              return

            this.data[key] = value
          },
          append(key, value) {
            if (key === '__proto__' || key === 'constructor' || key === 'prototype')
              return

            this.data[key] = value
          },
          delete(key) {
            if (hasOwn.call(this.data, key))
              delete this.data[key]
          },
          entries() {
            return Object.keys(this.data)
              .filter(k => hasOwn.call(this.data, k))
              .map(k => [k, this.data[k]])
          },
          keys() {
            return Object.keys(this.data).filter(k => hasOwn.call(this.data, k))
          },
          values() {
            return Object.keys(this.data)
              .filter(k => hasOwn.call(this.data, k))
              .map(k => this.data[k])
          },
        }
        return formDataLike
      }

      return arg
    })

    const result = await fn(...processedArgs)
    return JSON.parse(JSON.stringify(result))
  }
  catch (error) {
    throw new Error(`Server action error: ${error.message || String(error)}`)
  }
})()
