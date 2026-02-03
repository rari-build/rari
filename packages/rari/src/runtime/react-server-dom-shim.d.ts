export function registerClientReference(
  clientReference: any,
  id: string,
  exportName: string,
): any

export function registerClientComponent(
  componentFunction: any,
  id: string,
  exportName: string,
): void

export function registerServerReference(
  serverReference: any,
  id: string,
  exportName: string,
): any

export function createClientModuleProxy(id: string): any

export const __CLIENT_REFERENCE_REGISTRY__: Map<string, any>
export const __SERVER_REFERENCE_REGISTRY__: Map<string, any>
