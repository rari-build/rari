interface ProgressIndicatorOptions {
  position?: 'top-right' | 'top-left' | 'bottom-right' | 'bottom-left'
  size?: 'small' | 'medium'
  color?: string
}

class HMRProgressIndicator {
  private element: HTMLDivElement | null = null
  private isVisible: boolean = false
  private hideTimeout: NodeJS.Timeout | null = null
  private options: Required<ProgressIndicatorOptions>

  constructor(options: ProgressIndicatorOptions = {}) {
    this.options = {
      position: options.position || 'bottom-right',
      size: options.size || 'small',
      color: options.color || '#3b82f6',
    }
  }

  show(): void {
    if (typeof document === 'undefined')
      return

    if (this.hideTimeout) {
      clearTimeout(this.hideTimeout)
      this.hideTimeout = null
    }

    if (this.isVisible && this.element) {
      return
    }

    if (!this.element) {
      this.createElement()
    }

    if (this.element) {
      this.element.style.opacity = '1'
      this.element.style.transform = 'scale(1)'
      this.isVisible = true
    }
  }

  hide(delay: number = 500): void {
    if (!this.isVisible || !this.element)
      return

    if (this.hideTimeout) {
      clearTimeout(this.hideTimeout)
    }

    this.hideTimeout = setTimeout(() => {
      if (this.element) {
        this.element.style.opacity = '0'
        this.element.style.transform = 'scale(0.8)'
        this.isVisible = false
      }
      this.hideTimeout = null
    }, delay)
  }

  private createElement(): void {
    if (typeof document === 'undefined')
      return

    this.element = document.createElement('div')
    this.element.id = 'rari-hmr-indicator'
    this.element.setAttribute('aria-live', 'polite')
    this.element.setAttribute('aria-label', 'HMR update in progress')

    this.applyStyles()

    const spinner = document.createElement('div')
    spinner.className = 'rari-hmr-spinner'
    this.applySpinnerStyles(spinner)

    const label = document.createElement('span')
    label.textContent = 'HMR'
    label.style.fontSize = this.options.size === 'small' ? '10px' : '12px'
    label.style.fontWeight = '600'
    label.style.color = this.options.color
    label.style.marginLeft = '6px'

    this.element.appendChild(spinner)
    this.element.appendChild(label)

    document.body.appendChild(this.element)

    this.addAnimationStyles()
  }

  private applyStyles(): void {
    if (!this.element)
      return

    const positions = {
      'top-right': { top: '16px', right: '16px' },
      'top-left': { top: '16px', left: '16px' },
      'bottom-right': { bottom: '16px', right: '16px' },
      'bottom-left': { bottom: '16px', left: '16px' },
    }

    const position = positions[this.options.position]

    Object.assign(this.element.style, {
      position: 'fixed',
      zIndex: '999999',
      display: 'flex',
      alignItems: 'center',
      padding: '8px 12px',
      backgroundColor: 'rgba(255, 255, 255, 0.95)',
      border: `1px solid ${this.options.color}`,
      borderRadius: '8px',
      boxShadow: '0 2px 8px rgba(0, 0, 0, 0.1)',
      opacity: '0',
      transform: 'scale(0.8)',
      transition: 'opacity 0.2s ease, transform 0.2s ease',
      pointerEvents: 'none',
      ...position,
    })
  }

  private applySpinnerStyles(spinner: HTMLDivElement): void {
    const size = this.options.size === 'small' ? '14px' : '16px'

    Object.assign(spinner.style, {
      width: size,
      height: size,
      border: `2px solid rgba(59, 130, 246, 0.2)`,
      borderTop: `2px solid ${this.options.color}`,
      borderRadius: '50%',
      animation: 'rari-hmr-spin 0.8s linear infinite',
    })
  }

  private addAnimationStyles(): void {
    if (typeof document === 'undefined')
      return

    if (document.getElementById('rari-hmr-styles'))
      return

    const style = document.createElement('style')
    style.id = 'rari-hmr-styles'
    style.textContent = `
      @keyframes rari-hmr-spin {
        0% { transform: rotate(0deg); }
        100% { transform: rotate(360deg); }
      }
    `
    document.head.appendChild(style)
  }

  destroy(): void {
    if (this.hideTimeout) {
      clearTimeout(this.hideTimeout)
      this.hideTimeout = null
    }

    if (this.element && this.element.parentNode) {
      this.element.parentNode.removeChild(this.element)
    }

    this.element = null
    this.isVisible = false
  }
}

let indicatorInstance: HMRProgressIndicator | null = null

export function getHMRProgressIndicator(): HMRProgressIndicator {
  if (!indicatorInstance) {
    indicatorInstance = new HMRProgressIndicator({
      position: 'bottom-right',
      size: 'small',
      color: '#3b82f6',
    })
  }
  return indicatorInstance
}

export function showHMRProgress(): void {
  getHMRProgressIndicator().show()
}

export function hideHMRProgress(delay?: number): void {
  getHMRProgressIndicator().hide(delay)
}

export function destroyHMRProgress(): void {
  if (indicatorInstance) {
    indicatorInstance.destroy()
    indicatorInstance = null
  }
}
