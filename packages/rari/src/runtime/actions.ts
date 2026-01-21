import './csrf'

export interface ServerActionResponse {
  success: boolean
  result?: any
  error?: string
  redirect?: string
}

export interface ServerActionOptions {
  onSuccess?: (result: any) => void
  onError?: (error: string) => void
  onRedirect?: (url: string) => void
}

export function createServerReference(
  functionName: string,
  moduleId: string,
  exportName: string,
): (...args: any[]) => Promise<any> {
  return async (...args: any[]) => {
    try {
      const serializedArgs = args.map((arg) => {
        if (arg instanceof FormData) {
          const obj: Record<string, any> = {}
          arg.forEach((value, key) => {
            obj[key] = value
          })
          return obj
        }
        return arg
      })

      const fetchFn = typeof window !== 'undefined' && (window as any).fetchWithCsrf
        ? (window as any).fetchWithCsrf
        : fetch

      const response = await fetchFn('/_rari/action', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          id: moduleId,
          export_name: exportName,
          args: serializedArgs,
        }),
      })

      if (!response.ok) {
        const errorText = await response.text().catch(() => response.statusText)
        console.error(`[rari] ServerAction: HTTP ${response.status} error:`, errorText)
        throw new Error(
          `Server action "${exportName}" failed with status ${response.status}: ${errorText}`,
        )
      }

      const result: ServerActionResponse = await response.json()

      if (result.redirect) {
        if (typeof window !== 'undefined')
          window.location.href = result.redirect
        return { redirect: result.redirect }
      }

      if (!result.success) {
        const errorMsg = result.error || 'Server action failed without error message'
        console.error(`[rari] ServerAction: Action "${exportName}" failed:`, errorMsg)
        throw new Error(errorMsg)
      }

      return result.result
    }
    catch (error) {
      console.error(`[rari] ServerAction: Error executing "${exportName}":`, {
        moduleId,
        exportName,
        error: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined,
      })
      throw error
    }
  }
}

export function enhanceFormWithAction(
  form: HTMLFormElement,
  action: (formData: FormData) => Promise<any>,
  options: ServerActionOptions = {},
): () => void {
  const handleSubmit = async (event: Event) => {
    event.preventDefault()

    const formData = new FormData(form)

    try {
      const result = await action(formData)

      if (result && result.redirect) {
        if (options.onRedirect)
          options.onRedirect(result.redirect)

        if (typeof window !== 'undefined')
          window.location.href = result.redirect

        return
      }

      if (options.onSuccess)
        options.onSuccess(result)

      form.reset()
    }
    catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error'

      if (options.onError) {
        options.onError(errorMessage)
      }
      else {
        console.error('Server action error:', errorMessage)
      }
    }
  }

  form.addEventListener('submit', handleSubmit)

  return () => {
    form.removeEventListener('submit', handleSubmit)
  }
}

export function createFormAction(
  moduleId: string,
  exportName: string,
  action: (formData: FormData) => Promise<any>,
): {
  action: string
  enhance: (form: HTMLFormElement, options?: ServerActionOptions) => () => void
} {
  return {
    action: '/_rari/form-action',

    enhance: (form: HTMLFormElement, options: ServerActionOptions = {}) => {
      const actionIdInput = document.createElement('input')
      actionIdInput.type = 'hidden'
      actionIdInput.name = '__action_id'
      actionIdInput.value = moduleId
      form.appendChild(actionIdInput)

      const exportNameInput = document.createElement('input')
      exportNameInput.type = 'hidden'
      exportNameInput.name = '__export_name'
      exportNameInput.value = exportName
      form.appendChild(exportNameInput)

      if (typeof window !== 'undefined' && (window as any).getCsrfToken) {
        const csrfToken = (window as any).getCsrfToken()
        if (csrfToken) {
          let csrfInput = form.querySelector('input[name="__csrf_token"]') as HTMLInputElement
          if (!csrfInput) {
            csrfInput = document.createElement('input')
            csrfInput.type = 'hidden'
            csrfInput.name = '__csrf_token'
            form.appendChild(csrfInput)
          }
          csrfInput.value = csrfToken
        }
      }

      form.action = '/_rari/form-action'
      form.method = 'POST'

      return enhanceFormWithAction(form, action, options)
    },
  }
}

export function bindServerActions(): void {
  const forms = document.querySelectorAll('form[data-server-action]')

  forms.forEach((form) => {
    if (!(form instanceof HTMLFormElement))
      return

    const actionData = form.dataset.serverAction
    if (!actionData)
      return

    try {
      const { moduleId, exportName } = JSON.parse(actionData)

      const serverAction = createServerReference('action', moduleId, exportName)

      enhanceFormWithAction(form, async (formData) => {
        return await serverAction(formData)
      })
    }
    catch (error) {
      console.error('Failed to bind server action to form:', error)
    }
  })
}

if (typeof window !== 'undefined') {
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', bindServerActions)
  }
  else {
    bindServerActions()
  }
}
