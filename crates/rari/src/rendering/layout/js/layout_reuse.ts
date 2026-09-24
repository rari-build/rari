// oxlint-disable typescript/prefer-readonly-parameter-types
/// <reference path="../../types.d.ts" />

;(function initLayoutReuse() {
  if (typeof g['~rari']?.wrapLayoutReuse === 'function') return

  const LAYOUT_REUSE_ELEMENT = 'rari-layout-reuse'

  function requireCreateElement(): (
    component: unknown,
    props: unknown,
    ...children: readonly unknown[]
  ) => unknown {
    const react = g.React
    if (react == null || typeof react.createElement !== 'function') {
      throw new TypeError('[rari] React.createElement is not available')
    }
    return react.createElement
  }

  function wrapLayoutReuse(path: string, child: unknown, expandDocument: boolean): unknown {
    const createElement = requireCreateElement()
    const props: Record<string, unknown> = { 'data-rari-layout-path': path }
    if (expandDocument) props['data-rari-document-reuse'] = true
    return createElement(LAYOUT_REUSE_ELEMENT, props, child)
  }

  const rari = (g['~rari'] ??= {})
  rari.requireCreateElement = requireCreateElement
  rari.wrapLayoutReuse = wrapLayoutReuse
})()
