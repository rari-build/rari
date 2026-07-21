// eslint-disable-next-line unused-imports/no-unused-vars
function rariCreateHtmlBoundaryTracker() {
  let htmlState = 'outside'
  let pendingTagText = ''
  let pendingClosePrefix = ''
  let pendingRawTextClose = ''

  function reset() {
    htmlState = 'outside'
    pendingTagText = ''
    pendingClosePrefix = ''
    pendingRawTextClose = ''
  }

  function safeToInjectFlight() {
    return htmlState === 'outside'
  }

  function trackHtmlBoundaries(text: string) {
    let work = text
    if (htmlState === 'in_tag' && pendingTagText) {
      work = pendingTagText + text
      pendingTagText = ''
    }
    else if (
      (htmlState === 'in_inline_script' || htmlState === 'in_raw_text')
      && pendingClosePrefix
    ) {
      work = pendingClosePrefix + text
      pendingClosePrefix = ''
    }

    let i = 0
    const lower = work.toLowerCase()

    while (i < work.length) {
      switch (htmlState) {
        case 'outside': {
          const openAt = lower.indexOf('<', i)
          if (openAt === -1)
            return true
          htmlState = 'in_tag'
          i = openAt
          break
        }
        case 'in_tag': {
          const closeAt = work.indexOf('>', i)
          if (closeAt === -1) {
            pendingTagText = work.slice(i)
            return false
          }
          const openTag = work.slice(i, closeAt + 1)
          pendingTagText = ''
          const rawTextTag = /^<(style|title|textarea|xmp)\b/i.exec(openTag)
          if (rawTextTag) {
            htmlState = 'in_raw_text'
            pendingRawTextClose = `</${rawTextTag[1].toLowerCase()}>`
          }
          else {
            const isInlineScript = /^<script/i.test(openTag) && !/\bsrc\s*=/.test(openTag)
            htmlState = isInlineScript ? 'in_inline_script' : 'outside'
          }
          i = closeAt + 1
          break
        }
        case 'in_raw_text': {
          const closeTag = pendingRawTextClose
          const closeAt = lower.indexOf(closeTag, i)
          if (closeAt === -1) {
            const maxKeep = Math.max(closeTag.length - 1, 0)
            pendingClosePrefix = work.slice(Math.max(i, work.length - maxKeep))
            return false
          }
          htmlState = 'outside'
          pendingRawTextClose = ''
          pendingClosePrefix = ''
          i = closeAt + closeTag.length
          break
        }
        case 'in_inline_script': {
          const closeAt = lower.indexOf('</script>', i)
          if (closeAt === -1) {
            const maxKeep = '</script>'.length - 1
            pendingClosePrefix = work.slice(Math.max(i, work.length - maxKeep))
            return false
          }
          htmlState = 'outside'
          pendingClosePrefix = ''
          i = closeAt + 9
          break
        }
      }
    }

    return safeToInjectFlight()
  }

  return {
    reset,
    safeToInjectFlight,
    trackHtmlBoundaries,
    getState: () => htmlState,
  }
}
