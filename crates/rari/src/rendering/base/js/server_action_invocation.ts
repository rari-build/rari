/// <reference path="../../types.d.ts" />

(async () => {
  try {
    if (!g.getServerFunction)
      throw new TypeError('Server function registry not initialized (~serverFunctions from init_rsc_modules.js has not run)')

    const fn = g.getServerFunction('{function_name}')

    if (!fn)
      throw new TypeError('Server function \'{function_name}\' not found or not registered')
    if (typeof fn !== 'function')
      throw new TypeError('Server function \'{function_name}\' is not a function')

    const hasOwn = Object.prototype.hasOwnProperty

    const rawArgs = {args_json}
    // @ts-expect-error - Runtime template placeholder replaced by Rust, rawArgs will be an array
    const processedArgs = rawArgs.map((arg: unknown) => {
      if (arg && typeof arg === 'object' && !Array.isArray(arg) && !(arg instanceof FormData)) {
        const data = arg as Record<string, unknown>
        const formDataLike = {
          data,
          get(key: string) {
            if (hasOwn.call(this.data, key))
              return this.data[key]

            return undefined
          },
          has(key: string) {
            return hasOwn.call(this.data, key)
          },
          set(key: string, value: unknown) {
            if (key === '__proto__' || key === 'constructor' || key === 'prototype')
              return

            this.data[key] = value
          },
          append(key: string, value: unknown) {
            if (key === '__proto__' || key === 'constructor' || key === 'prototype')
              return

            this.data[key] = value
          },
          delete(key: string) {
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
  catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error)
    throw new Error(`Server action error: ${errorMessage}`)
  }
})()
