/**
 * Inject React edge client's form-action helpers into the browser flight client.
 * Edge provides $$FORM_ACTION encoding; browser client only has callServer RPC.
 */

function replaceAnchoredBlock(
  source: string,
  anchor: string,
  replacement: string,
  context: string,
): string {
  const first = source.indexOf(anchor)
  if (first === -1) {
    throw new Error(
      `Failed to patch flight browser client: anchor for ${context} not found. ` +
        `The react-server-dom-webpack sources likely changed shape after a version bump; ` +
        `update the anchors in patch-flight-browser-client.ts.`,
    )
  }
  if (source.includes(anchor, first + 1)) {
    throw new Error(
      `Failed to patch flight browser client: anchor for ${context} matched more than once. ` +
        `Update the anchors in patch-flight-browser-client.ts to be unambiguous.`,
    )
  }
  return source.slice(0, first) + replacement + source.slice(first + anchor.length)
}

export function patchBrowserClientForFormActions(
  browserSource: string,
  edgeSource: string,
): string {
  const helpersStart = edgeSource.indexOf('var boundCache = new WeakMap();')
  const bindEnd = edgeSource.indexOf('function createBoundServerReference', helpersStart)
  if (helpersStart === -1 || bindEnd === -1) {
    throw new Error('Failed to locate edge client form-action helpers for browser patch')
  }

  const formActionBlock = replaceAnchoredBlock(
    edgeSource.slice(helpersStart, bindEnd),
    `return {
    name: referenceClosure,
    method: "POST",
    encType: "multipart/form-data",
    data: data
  };`,
    `function resolveRariFormActionUrl() {
    var g = typeof globalThis !== "undefined" ? globalThis : {};
    var rari = g["~rari"];
    if (rari && rari.actionPostUrl)
      return rari.actionPostUrl;
    if (typeof window !== "undefined")
      return window.location.pathname + window.location.search;
    return "/";
  }
  return {
    name: referenceClosure,
    method: "POST",
    encType: "multipart/form-data",
    action: resolveRariFormActionUrl(),
    data: data
  };`,
    'edge $$FORM_ACTION return block',
  )

  return replaceAnchoredBlock(
    browserSource,
    `function registerBoundServerReference(reference, id, bound) {
  knownServerReferences.has(reference) ||
    knownServerReferences.set(reference, {
      id: id,
      originalBind: reference.bind,
      bound: bound
    });
}`,
    formActionBlock,
    'browser registerBoundServerReference',
  )
}
