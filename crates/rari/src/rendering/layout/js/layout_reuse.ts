// oxlint-disable typescript/prefer-readonly-parameter-types
/// <reference path="../../types.d.ts" />

;(function initLayoutReuse() {
  if (
    typeof g['~rari']?.wrapLayoutReuse === 'function' &&
    typeof g['~rari']?.stampLayoutPath === 'function'
  ) {
    return
  }

  const LAYOUT_REUSE_ELEMENT = 'rari-layout-reuse'
  const LAYOUT_STAMP_ELEMENT = 'rari-layout-stamp'
  const LAYOUT_PATH_PROP = 'data-rari-layout-path'
  const DISPLAY_CONTENTS = { display: 'contents' }

  function requireCreateElement(): (
    component: unknown,
    props: unknown,
    ...children: readonly unknown[]
  ) => unknown {
    const react = g.React
    if (react == null || typeof react.createElement !== 'function') {
      throw new TypeError('[rari] createElement is not available')
    }
    return react.createElement
  }

  function isRecord(value: unknown): value is Record<string, unknown> {
    return value != null && typeof value === 'object'
  }

  function alreadyStamped(child: unknown, path: string): boolean {
    if (!isRecord(child)) return false
    const type = Reflect.get(child, 'type')
    if (type === LAYOUT_REUSE_ELEMENT || type === LAYOUT_STAMP_ELEMENT) {
      const props = Reflect.get(child, 'props')
      return isRecord(props) && props[LAYOUT_PATH_PROP] === path
    }
    const props = Reflect.get(child, 'props')
    return isRecord(props) && props[LAYOUT_PATH_PROP] === path
  }

  function stampLayoutPath(path: string, child: unknown): unknown {
    if (path === '' || alreadyStamped(child, path)) return child
    return requireCreateElement()(
      LAYOUT_STAMP_ELEMENT,
      { [LAYOUT_PATH_PROP]: path, style: DISPLAY_CONTENTS },
      child,
    )
  }

  function wrapLayoutReuse(path: string, child: unknown, expandDocument: boolean): unknown {
    const createElement = requireCreateElement()
    const props: Record<string, unknown> = { [LAYOUT_PATH_PROP]: path }
    if (expandDocument) props['data-rari-document-reuse'] = true
    return createElement(LAYOUT_REUSE_ELEMENT, props, child)
  }

  const rari = (g['~rari'] ??= {})
  rari.requireCreateElement = requireCreateElement
  rari.wrapLayoutReuse = wrapLayoutReuse
  rari.stampLayoutPath = stampLayoutPath
})()
