export function hasFizzMarkers(root: Element): boolean {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_COMMENT)
  while (walker.nextNode()) {
    const comment = walker.currentNode as Comment
    if (comment.data === '$' || comment.data === '$?' || comment.data === '/$')
      return true
  }

  if (root.querySelector('[data-reactroot]'))
    return true

  if (root.querySelectorAll('template[data-rri]').length > 0)
    return true

  return false
}

export function clearServerInjectedErrors(root: Element): void {
  root.querySelectorAll('.rari-error:not([data-rari-hydration-failure])').forEach((element) => {
    element.remove()
  })
}
