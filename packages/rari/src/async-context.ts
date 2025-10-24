interface RequestContext {
  headers: Record<string, string>
  pathname: string
}

let currentContext: RequestContext | null = null

export function setRequestContext(context: RequestContext) {
  currentContext = context
}

export function getRequestContext(): RequestContext | null {
  return currentContext
}

export function clearRequestContext() {
  currentContext = null
}

export async function headers(): Promise<Headers> {
  const context = getRequestContext()
  if (!context) {
    return new Headers()
  }

  const headersObj = new Headers()
  for (const [key, value] of Object.entries(context.headers)) {
    headersObj.set(key, value)
  }

  headersObj.set('x-pathname', context.pathname)

  return headersObj
}
