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

  if (typeof Component !== 'function') {
    throw new Error('Component ${registryKey}#${exportName} is not a function, got: ' + typeof Component);
  }

  return Component(props);
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
