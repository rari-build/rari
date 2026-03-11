if (!globalThis['~rari'])
  globalThis['~rari'] = {}

globalThis['~rari'].apiHandler = {
  async callHandler(requestData, moduleSpecifier, methodName) {
    try {
      const url = new URL(requestData.url, 'http://localhost')

      const headers = new Headers(requestData.headers || {})

      const method = requestData.method.toUpperCase()
      const body = (method === 'GET' || method === 'HEAD') ? undefined : (requestData.body || undefined)

      const request = new Request(url.toString(), {
        method: requestData.method,
        headers,
        body,
      })

      const context = {
        params: requestData.params || {},
      }

      const moduleNamespace = await import(moduleSpecifier)
      const handler = moduleNamespace[methodName]

      if (typeof handler !== 'function') {
        const available = Object.keys(moduleNamespace).join(', ')
        throw new Error(
          `Handler '${methodName}' is not a function. Available exports: ${available}`,
        )
      }

      const result = await handler(request, context)

      if (result instanceof Response) {
        const body = await result.text()
        const responseHeaders = {}
        result.headers.forEach((value, key) => {
          responseHeaders[key] = value
        })

        return {
          status: result.status,
          statusText: result.statusText,
          headers: responseHeaders,
          body,
        }
      }
      else {
        try {
          return {
            status: 200,
            statusText: 'OK',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify(result),
          }
        }
        catch (serializationError) {
          console.error('Failed to serialize API response:', serializationError)
          const isDevelopment = globalThis['~rari']?.isDevelopment === true
          return {
            status: 500,
            statusText: 'Internal Server Error',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({
              error: 'Failed to serialize response',
              message: serializationError?.message || 'Response contains circular references or non-serializable values',
              stack: isDevelopment ? serializationError?.stack : undefined,
            }),
          }
        }
      }
    }
    catch (error) {
      console.error('API route handler error:', error)
      const isDevelopment = globalThis['~rari']?.isDevelopment === true
      return {
        status: 500,
        statusText: 'Internal Server Error',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          error: 'Internal Server Error',
          message: error?.message || String(error),
          stack: isDevelopment ? error?.stack : undefined,
        }),
      }
    }
  },
}
