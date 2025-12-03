/* eslint-disable no-undef, style/object-curly-spacing */
(async () => {
  try {
    const fn = globalThis['{function_name}']
    if (typeof fn !== 'function') {
      throw new TypeError('Function \'{function_name}\' not found in globalThis')
    }

    const rawArgs = {args_json}
    const processedArgs = rawArgs.map((arg) => {
      if (arg && typeof arg === 'object' && !Array.isArray(arg) && !(arg instanceof FormData)) {
        const formDataLike = {
          data: arg,
          get(key) { return this.data[key] },
          has(key) { return key in this.data },
          set(key, value) { this.data[key] = value },
          append(key, value) { this.data[key] = value },
          delete(key) { delete this.data[key] },
          entries() { return Object.entries(this.data) },
          keys() { return Object.keys(this.data) },
          values() { return Object.values(this.data) },
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
