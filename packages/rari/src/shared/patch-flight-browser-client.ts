/**
 * Inject React edge client's form-action helpers into the browser flight client.
 * Edge provides $$FORM_ACTION encoding; browser client only has callServer RPC.
 */
export function patchBrowserClientForFormActions(
  browserSource: string,
  edgeSource: string,
): string {
  const helpersStart = edgeSource.indexOf('var boundCache = new WeakMap();')
  const bindEnd = edgeSource.indexOf('function createBoundServerReference', helpersStart)
  if (helpersStart === -1 || bindEnd === -1) {
    throw new Error('Failed to locate edge client form-action helpers for browser patch')
  }

  const formActionBlock = edgeSource.slice(helpersStart, bindEnd).replace(
    `return {
    name: referenceClosure,
    method: "POST",
    encType: "multipart/form-data",
    data: data
  };`,
    `return {
    name: referenceClosure,
    method: "POST",
    encType: "multipart/form-data",
    action: "/_rari/action",
    data: data
  };`,
  )

  return browserSource.replace(
    `function registerBoundServerReference(reference, id, bound) {
  knownServerReferences.has(reference) ||
    knownServerReferences.set(reference, {
      id: id,
      originalBind: reference.bind,
      bound: bound
    });
}`,
    formActionBlock,
  )
}

/** Rolldown collapses $$FORM_ACTION to $FORM_ACTION in property keys. */
export function fixRolldownDoubleDollarProperties(code: string): string {
  return code
    .replace(/(?<!\$)\$FORM_ACTION/g, () => '$$FORM_ACTION')
    .replace(/(?<!\$)\$IS_SIGNATURE_EQUAL/g, () => '$$IS_SIGNATURE_EQUAL')
}
