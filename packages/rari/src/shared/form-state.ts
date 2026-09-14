// oxlint-disable-next-line typescript/prefer-readonly-parameter-types mutates form control values in place
export function applyFormDataToForm(form: HTMLFormElement, formData: FormData): boolean {
  let allSucceeded = true
  const processedKeys = new Set<string>()

  for (const key of formData.keys()) {
    if (processedKeys.has(key)) continue
    processedKeys.add(key)

    try {
      const values = formData.getAll(key)
      const stringValues = values.filter((value): value is string => typeof value === 'string')
      const valueSet = new Set(stringValues)
      const elements = form.elements.namedItem(key)

      if (elements instanceof RadioNodeList) {
        elements.forEach(element => {
          if (!(element instanceof HTMLInputElement)) return

          if (element.type === 'radio' || element.type === 'checkbox')
            element.checked = valueSet.has(element.value)
          else if (stringValues.length === 1) element.value = stringValues[0]
        })
        continue
      }

      if (elements instanceof HTMLSelectElement && elements.multiple) {
        for (const option of elements.options) {
          option.selected = valueSet.has(option.value)
        }
        continue
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
          elements.checked = valueSet.has(elements.value)
        else if (stringValues.length > 0) elements.value = stringValues[0]
      }
    } catch {
      allSucceeded = false
    }
  }

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
