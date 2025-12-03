/* eslint-disable no-undef, style/object-curly-spacing */
(async () => {
  try {
    const hasOwn = Object.prototype.hasOwnProperty
    if (!hasOwn.call(globalThis, '{function_name}')) {
      throw new TypeError('Function \'{function_name}\' not found in globalThis')
    }

    const fn = globalThis['{function_name}']
    if (typeof fn !== 'function') {
      throw new TypeError('Function \'{function_name}\' is not a function')
    }

    const rawArgs = {args_json}
    const processedArgs = rawArgs.map((arg) => {
      if (arg && typeof arg === 'object' && !Array.isArray(arg) && !(arg instanceof FormData)) {
        const formDataLike = {
          data: arg,
          get(key) {
            if (hasOwn.call(this.data, key)) {
              return this.data[key]
            }
            return undefined
          },
          has(key) {
            return hasOwn.call(this.data, key)
          },
          set(key, value) {
            if (key === '__proto__' || key === 'constructor' || key === 'prototype') {
              return
            }
            this.data[key] = value
          },
          append(key, value) {
            if (key === '__proto__' || key === 'constructor' || key === 'prototype') {
              return
            }
            this.data[key] = value
          },
          delete(key) {
            if (hasOwn.call(this.data, key)) {
              delete this.data[key]
            }
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
