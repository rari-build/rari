export function hasFizzMarkers(root: Element): boolean {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_COMMENT)
  while (walker.nextNode()) {
    const node = walker.currentNode
    if (node.nodeType === 8 && 'data' in node) {
      const data = node.data
      if (data === '$' || data === '$?' || data === '/$') return true
    }
  }

  if (root.querySelector('[data-reactroot]')) return true

  if (root.querySelectorAll('template[data-rri]').length > 0) return true

  return false
}

function isIgnorableLeadingNode(el: Element): boolean {
  const tag = el.tagName
  if (
    tag === 'SCRIPT' ||
    tag === 'STYLE' ||
    tag === 'LINK' ||
    tag === 'META' ||
    tag === 'NOSCRIPT'
  ) {
    return true
  }
  const getAttr = typeof el.getAttribute === 'function' ? el.getAttribute.bind(el) : null
  if (getAttr == null) return false
  return getAttr('data-rari-nav-transition') != null || getAttr('aria-hidden') === 'true'
}

function firstMeaningfulElementChild(root: Element): Element | null {
  let el = root.firstElementChild
  while (el != null && isIgnorableLeadingNode(el)) {
    el = el.nextElementSibling
  }
  return el
}

export function hasServerRenderedDom(root: Element): boolean {
  if (hasFizzMarkers(root)) return true

  const first = firstMeaningfulElementChild(root)
  if (first === null) return false

  return !first.classList.contains('rari-error')
}

export function shouldHydrateServerDom(root: Element): boolean {
  return hasFizzMarkers(root)
}

export function clearServerInjectedErrors(root: Element): void {
  root.querySelectorAll('.rari-error:not([data-rari-hydration-failure])').forEach(element => {
    element.remove()
  })
}
