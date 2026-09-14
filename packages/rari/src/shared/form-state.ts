// oxlint-disable-next-line typescript/prefer-readonly-parameter-types mutates form control values in place
export function applyFormDataToForm(form: HTMLFormElement, formData: FormData): boolean {
  let allSucceeded = true

  formData.forEach((value, key) => {
    try {
      const elements = form.elements.namedItem(key)

      if (elements instanceof RadioNodeList) {
        elements.forEach(element => {
          if (!(element instanceof HTMLInputElement)) return

          if (element.type === 'radio' || element.type === 'checkbox')
            element.checked = element.value === value
          else if (typeof value === 'string') element.value = value
        })
        return
      }

      if (
        elements instanceof HTMLInputElement ||
        elements instanceof HTMLTextAreaElement ||
        elements instanceof HTMLSelectElement
      ) {
        if (
          elements instanceof HTMLInputElement &&
          (elements.type === 'checkbox' || elements.type === 'radio')
        )
          elements.checked = elements.value === value
        else if (typeof value === 'string') elements.value = value
      }
    } catch {
      allSucceeded = false
    }
  })

  return allSucceeded
}

export function captureIndexedFormData(): Map<string, FormData> {
  const formDataMap = new Map<string, FormData>()
  if (typeof document === 'undefined') return formDataMap

  document.querySelectorAll('form').forEach((form, index) => {
    formDataMap.set(`form-${index}`, new FormData(form))
  })

  return formDataMap
}

export function restoreIndexedFormData(formDataMap: ReadonlyMap<string, FormData>): void {
  if (typeof document === 'undefined') return

  document.querySelectorAll('form').forEach((form, index) => {
    const savedData = formDataMap.get(`form-${index}`)
    if (!savedData) return
    applyFormDataToForm(form, savedData)
  })
}
