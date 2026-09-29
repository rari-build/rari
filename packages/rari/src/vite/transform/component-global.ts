export function buildGlobalClientComponentWrapper(
  bindingName: string,
  registryKey: string,
  exportName: string,
): string {
  const exportAccess =
    exportName === 'default'
      ? `if (Component && typeof Component === 'object' && Component.default) {
    Component = Component.default;
  }`
      : `if (Component && typeof Component === 'object') {
    Component = Component[${JSON.stringify(exportName)}] ?? Component.default?.[${JSON.stringify(exportName)}];
  }`

  return `// Component reference: ${registryKey}#${exportName}
const ${bindingName} = (props) => {
  let Component = globalThis['~clientComponents']?.[${JSON.stringify(registryKey)}]?.component
    || globalThis[${JSON.stringify(registryKey)}];

  ${exportAccess}

  if (!Component) {
    throw new Error('Component ${registryKey}#${exportName} not loaded');
  }

  if (typeof Component === 'function') {
    return Component(props);
  }
  if (typeof Component === 'symbol') {
    const key = Symbol.keyFor(Component);
    if (
      key !== 'react.fragment' &&
      key !== 'react.profiler' &&
      key !== 'react.strict_mode' &&
      key !== 'react.suspense' &&
      key !== 'react.suspense_list' &&
      key !== 'react.activity' &&
      key !== 'react.view_transition'
    ) {
      throw new Error('Component ${registryKey}#${exportName} is not a component type symbol');
    }
    const React = globalThis.React;
    if (React == null || typeof React.createElement !== 'function') {
      throw new Error('Component ${registryKey}#${exportName} needs React.createElement');
    }
    return React.createElement(Component, props);
  }
  if (typeof Component === 'object' && Component != null) {
    const type = Component.$$typeof;
    if (
      type === Symbol.for('react.element') ||
      type === Symbol.for('react.transitional.element') ||
      type === Symbol.for('react.portal')
    ) {
      throw new Error('Component ${registryKey}#${exportName} is a React element, not a component type');
    }
    if ('$$typeof' in Component || Component['~isClientComponent'] === true) {
      const React = globalThis.React;
      if (React == null || typeof React.createElement !== 'function') {
        throw new Error('Component ${registryKey}#${exportName} needs React.createElement');
      }
      return React.createElement(Component, props);
    }
  }

  throw new Error('Component ${registryKey}#${exportName} is not a component, got: ' + typeof Component);
}`
}

export function buildGlobalClientNamespaceWrapper(
  bindingName: string,
  registryKey: string,
): string {
  return `// Component namespace reference: ${registryKey}
const ${bindingName} = globalThis['~clientComponents']?.[${JSON.stringify(registryKey)}]?.component
  || globalThis[${JSON.stringify(registryKey)}]
  || {};`
}
