function rariCreateHtmlBoundaryTracker() {
  let htmlState = 'outside'
  let pendingTagText = ''
  let pendingClosePrefix = ''
  let pendingRawTextClose = ''
  let bodyOpened = false
  let bodyHasContent = false

  function reset() {
    htmlState = 'outside'
    pendingTagText = ''
    pendingClosePrefix = ''
    pendingRawTextClose = ''
    bodyOpened = false
    bodyHasContent = false
  }

  function safeToInjectFlight() {
    return htmlState === 'outside' && bodyHasContent
  }

  function prependPending(text: string): string {
    if (htmlState === 'in_tag' && pendingTagText) {
      const work = pendingTagText + text
      pendingTagText = ''
      return work
    }
    if ((htmlState === 'in_inline_script' || htmlState === 'in_raw_text') && pendingClosePrefix) {
      const work = pendingClosePrefix + text
      pendingClosePrefix = ''
      return work
    }
    return text
  }

  function processOutside(work: string, lower: string, start: number): number | boolean {
    const openAt = lower.indexOf('<', start)
    if (openAt === -1) {
      if (bodyOpened && work.slice(start).trim() !== '') bodyHasContent = true
      return true
    }
    if (bodyOpened && openAt > start && work.slice(start, openAt).trim() !== '') {
      bodyHasContent = true
    }
    htmlState = 'in_tag'
    return openAt
  }

  function processInTag(work: string, start: number): number | boolean {
    const closeAt = work.indexOf('>', start)
    if (closeAt === -1) {
      pendingTagText = work.slice(start)
      return false
    }
    const openTag = work.slice(start, closeAt + 1)
    pendingTagText = ''
    if (/^<body\b/i.test(openTag)) bodyOpened = true
    const rawTextTag = /^<(style|title|textarea|xmp)\b/i.exec(openTag)
    if (rawTextTag) {
      htmlState = 'in_raw_text'
      pendingRawTextClose = `</${rawTextTag[1].toLowerCase()}>`
    } else {
      const isInlineScript = /^<script/i.test(openTag) && !/\bsrc\s*=/.test(openTag)
      if (
        bodyOpened &&
        !isInlineScript &&
        !/^<\/?(?:script|style|link|meta|noscript)\b/i.test(openTag)
      ) {
        bodyHasContent = true
      }
      htmlState = isInlineScript ? 'in_inline_script' : 'outside'
    }
    return closeAt + 1
  }

  function processRawText(work: string, lower: string, start: number): number | boolean {
    const closeTag = pendingRawTextClose
    const closeAt = lower.indexOf(closeTag, start)
    if (closeAt === -1) {
      const maxKeep = Math.max(closeTag.length - 1, 0)
      pendingClosePrefix = work.slice(Math.max(start, work.length - maxKeep))
      return false
    }
    htmlState = 'outside'
    pendingRawTextClose = ''
    pendingClosePrefix = ''
    return closeAt + closeTag.length
  }

  function processInlineScript(work: string, lower: string, start: number): number | boolean {
    const closeAt = lower.indexOf('</script>', start)
    if (closeAt === -1) {
      const maxKeep = '</script>'.length - 1
      pendingClosePrefix = work.slice(Math.max(start, work.length - maxKeep))
      return false
    }
    htmlState = 'outside'
    pendingClosePrefix = ''
    return closeAt + 9
  }

  function trackHtmlBoundaries(text: string) {
    const work = prependPending(text)
    let i = 0
    const lower = work.toLowerCase()

    while (i < work.length) {
      let next: number | boolean
      switch (htmlState) {
        case 'outside':
          next = processOutside(work, lower, i)
          break
        case 'in_tag':
          next = processInTag(work, i)
          break
        case 'in_raw_text':
          next = processRawText(work, lower, i)
          break
        case 'in_inline_script':
          next = processInlineScript(work, lower, i)
          break
        default:
          return htmlState === 'outside'
      }
      if (typeof next === 'boolean') return next
      i = next
    }

    return htmlState === 'outside'
  }

  return {
    reset,
    safeToInjectFlight,
    trackHtmlBoundaries,
    getState: () => htmlState,
  }
}
