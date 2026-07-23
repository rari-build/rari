export interface ScrollPosition {
  readonly x: number
  readonly y: number
  readonly element: string
}

export interface PreservedState {
  readonly scrollPositions: Map<string, ScrollPosition>
  readonly formData: Map<string, FormData>
  readonly focusedElement: string | null
}

export interface StatePreserverConfig {
  readonly maxHistorySize?: number
  readonly scrollableSelector?: string
}

export class StatePreserver {
  private readonly stateHistory: Map<string, PreservedState>
  private routeAccessOrder: string[]
  private readonly maxHistorySize: number
  private readonly scrollableSelector: string

  constructor(config: StatePreserverConfig = {}) {
    this.stateHistory = new Map()
    this.routeAccessOrder = []
    this.maxHistorySize =
      config.maxHistorySize != null && config.maxHistorySize !== 0 ? config.maxHistorySize : 50
    this.scrollableSelector =
      config.scrollableSelector != null && config.scrollableSelector !== ''
        ? config.scrollableSelector
        : '[data-scrollable], .scrollable, [style*="overflow"]'
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

          if (
            element.scrollHeight > element.clientHeight ||
            element.scrollWidth > element.clientWidth
          ) {
            positions.set(elementId, {
              x: element.scrollLeft,
              y: element.scrollTop,
              element: elementId,
            })
          }
        }
      })
    } catch {}

    return positions
  }

  private captureFormData(): Map<string, FormData> {
    const formDataMap = new Map<string, FormData>()

    try {
      const forms = document.querySelectorAll('form')

      forms.forEach((form, index) => {
        const formId = form.id || form.name || `form-${index}`

        const formData = new FormData(form)

        if (!formData.entries().next().done) formDataMap.set(formId, formData)
      })
    } catch {}

    return formDataMap
  }

  private captureFocusedElement(): string | null {
    try {
      const activeElement = document.activeElement

      if (activeElement && activeElement !== document.body) {
        if (activeElement.id) return `#${activeElement.id}`

        if (
          activeElement instanceof HTMLInputElement ||
          activeElement instanceof HTMLTextAreaElement
        ) {
          if (activeElement.name) return `[name="${activeElement.name}"]`
        }
      }
    } catch {}

    return null
  }

  private storeState(route: string, state: PreservedState): void {
    const existingIndex = this.routeAccessOrder.indexOf(route)
    if (existingIndex !== -1) this.routeAccessOrder.splice(existingIndex, 1)

    this.routeAccessOrder.push(route)
    this.stateHistory.set(route, state)

    while (this.routeAccessOrder.length > this.maxHistorySize) {
      const oldestRoute = this.routeAccessOrder.shift()
      if (oldestRoute != null && oldestRoute !== '') this.stateHistory.delete(oldestRoute)
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
    if (index !== -1) this.routeAccessOrder.splice(index, 1)
  }

  public restoreState(route: string): boolean {
    const state = this.stateHistory.get(route)

    if (!state) return false

    let success = true

    if (!this.restoreScrollPositions(state.scrollPositions)) success = false
    if (!this.restoreFormData(state.formData)) success = false
    if (state.focusedElement != null && state.focusedElement !== '')
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
          } else {
            const element =
              document.getElementById(key) ??
              document.querySelector(`[data-scrollable-id="${key}"]`)

            if (element instanceof HTMLElement) {
              element.scrollLeft = position.x
              element.scrollTop = position.y
            } else {
              allSucceeded = false
            }
          }
        } catch {
          allSucceeded = false
        }
      })
    } catch (error) {
      console.error('[rari] Router: Failed to restore scroll positions:', error)
      allSucceeded = false
    }

    return allSucceeded
  }

  private restoreFormData(formDataMap: Map<string, FormData>): boolean {
    let allSucceeded = true

    try {
      formDataMap.forEach((formData, formId) => {
        try {
          const formById = document.getElementById(formId)
          const form =
            formById instanceof HTMLFormElement
              ? formById
              : (document.querySelector(`form[name="${formId}"]`) ??
                document.querySelectorAll('form')[Number.parseInt(formId.replace('form-', ''), 10)])

          if (form instanceof HTMLFormElement) {
            formData.forEach((value, key) => {
              try {
                const elements = form.elements.namedItem(key)

                if (elements instanceof RadioNodeList) {
                  elements.forEach(element => {
                    if (element instanceof HTMLInputElement) {
                      if (element.type === 'radio' || element.type === 'checkbox')
                        element.checked = element.value === value
                      else if (typeof value === 'string') element.value = value
                    }
                  })
                } else if (
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
          } else {
            allSucceeded = false
          }
        } catch {
          allSucceeded = false
        }
      })
    } catch (error) {
      console.error('[rari] Router: Failed to restore form data:', error)
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
          } catch {}
        })
      }
    } catch {}
  }
}
