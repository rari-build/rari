export interface ScrollPosition {
  x: number
  y: number
  element: string
}

export interface PreservedState {
  scrollPositions: Map<string, ScrollPosition>
  formData: Map<string, FormData>
  focusedElement: string | null
}

export interface StatePreserverConfig {
  maxHistorySize?: number
  scrollableSelector?: string
}

export class StatePreserver {
  private stateHistory: Map<string, PreservedState>
  private routeAccessOrder: string[]
  private maxHistorySize: number
  private scrollableSelector: string

  constructor(config: StatePreserverConfig = {}) {
    this.stateHistory = new Map()
    this.routeAccessOrder = []
    this.maxHistorySize = config.maxHistorySize || 50
    this.scrollableSelector = config.scrollableSelector || '[data-scrollable], .scrollable, [style*="overflow"]'
  }

  public captureState(route: string): PreservedState {
    const state: PreservedState = {
      scrollPositions: this.captureScrollPositions(),
      formData: this.captureFormData(),
      focusedElement: this.captureFocusedElement(),
    }

    this.storeState(route, state)

    return state
  }

  private captureScrollPositions(): Map<string, ScrollPosition> {
    const positions = new Map<string, ScrollPosition>()

    try {
      positions.set('window', {
        x: window.scrollX,
        y: window.scrollY,
        element: 'window',
      })

      const scrollableElements = document.querySelectorAll(this.scrollableSelector)

      scrollableElements.forEach((element, index) => {
        if (element instanceof HTMLElement) {
          const elementId = element.id || `scrollable-${index}`

          if (element.scrollHeight > element.clientHeight || element.scrollWidth > element.clientWidth) {
            positions.set(elementId, {
              x: element.scrollLeft,
              y: element.scrollTop,
              element: elementId,
            })
          }
        }
      })
    }
    catch {}

    return positions
  }

  private captureFormData(): Map<string, FormData> {
    const formDataMap = new Map<string, FormData>()

    try {
      const forms = document.querySelectorAll('form')

      forms.forEach((form, index) => {
        const formId = form.id || form.name || `form-${index}`

        const formData = new FormData(form)

        if (Array.from(formData.entries()).length > 0)
          formDataMap.set(formId, formData)
      })
    }
    catch {}

    return formDataMap
  }

  private captureFocusedElement(): string | null {
    try {
      const activeElement = document.activeElement

      if (activeElement && activeElement !== document.body) {
        if (activeElement.id)
          return `#${activeElement.id}`

        if (activeElement instanceof HTMLInputElement || activeElement instanceof HTMLTextAreaElement) {
          if (activeElement.name)
            return `[name="${activeElement.name}"]`
        }
      }
    }
    catch {}

    return null
  }

  private storeState(route: string, state: PreservedState): void {
    const existingIndex = this.routeAccessOrder.indexOf(route)
    if (existingIndex !== -1)
      this.routeAccessOrder.splice(existingIndex, 1)

    this.routeAccessOrder.push(route)
    this.stateHistory.set(route, state)

    while (this.routeAccessOrder.length > this.maxHistorySize) {
      const oldestRoute = this.routeAccessOrder.shift()
      if (oldestRoute)
        this.stateHistory.delete(oldestRoute)
    }
  }

  public getHistorySize(): number {
    return this.stateHistory.size
  }

  public hasState(route: string): boolean {
    return this.stateHistory.has(route)
  }

  public getState(route: string): PreservedState | undefined {
    return this.stateHistory.get(route)
  }

  public clearAll(): void {
    this.stateHistory.clear()
    this.routeAccessOrder = []
  }

  public clearState(route: string): void {
    this.stateHistory.delete(route)
    const index = this.routeAccessOrder.indexOf(route)
    if (index !== -1)
      this.routeAccessOrder.splice(index, 1)
  }

  public restoreState(route: string): boolean {
    const state = this.stateHistory.get(route)

    if (!state)
      return false

    let success = true

    if (!this.restoreScrollPositions(state.scrollPositions))
      success = false
    if (!this.restoreFormData(state.formData))
      success = false
    if (state.focusedElement)
      this.restoreFocus(state.focusedElement)

    return success
  }

  private restoreScrollPositions(positions: Map<string, ScrollPosition>): boolean {
    let allSucceeded = true

    try {
      positions.forEach((position, key) => {
        try {
          if (key === 'window') {
            window.scrollTo(position.x, position.y)
          }
          else {
            const element = document.getElementById(key) || document.querySelector(`[data-scrollable-id="${key}"]`)

            if (element instanceof HTMLElement) {
              element.scrollLeft = position.x
              element.scrollTop = position.y
            }
            else {
              allSucceeded = false
            }
          }
        }
        catch {
          allSucceeded = false
        }
      })
    }
    catch (error) {
      console.error('Failed to restore scroll positions:', error)
      allSucceeded = false
    }

    return allSucceeded
  }

  private restoreFormData(formDataMap: Map<string, FormData>): boolean {
    let allSucceeded = true

    try {
      formDataMap.forEach((formData, formId) => {
        try {
          const form = document.getElementById(formId) as HTMLFormElement
            || document.querySelector(`form[name="${formId}"]`) as HTMLFormElement
            || document.querySelectorAll('form')[Number.parseInt(formId.replace('form-', ''), 10)]

          if (form instanceof HTMLFormElement) {
            formData.forEach((value, key) => {
              try {
                const elements = form.elements.namedItem(key)

                if (elements instanceof RadioNodeList) {
                  elements.forEach((element) => {
                    if (element instanceof HTMLInputElement) {
                      if (element.type === 'radio' || element.type === 'checkbox') {
                        element.checked = element.value === value
                      }
                      else {
                        element.value = value as string
                      }
                    }
                  })
                }
                else if (elements instanceof HTMLInputElement || elements instanceof HTMLTextAreaElement || elements instanceof HTMLSelectElement) {
                  if (elements instanceof HTMLInputElement && (elements.type === 'checkbox' || elements.type === 'radio')) {
                    elements.checked = elements.value === value
                  }
                  else {
                    elements.value = value as string
                  }
                }
              }
              catch {
                allSucceeded = false
              }
            })
          }
          else {
            allSucceeded = false
          }
        }
        catch {
          allSucceeded = false
        }
      })
    }
    catch (error) {
      console.error('Failed to restore form data:', error)
      allSucceeded = false
    }

    return allSucceeded
  }

  private restoreFocus(selector: string): void {
    try {
      const element = document.querySelector(selector)

      if (element instanceof HTMLElement) {
        requestAnimationFrame(() => {
          try {
            element.focus()
          }
          catch {}
        })
      }
    }
    catch {}
  }
}
