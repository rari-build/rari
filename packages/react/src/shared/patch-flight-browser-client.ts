/**
 * Inject React edge client's form-action helpers into the browser flight client.
 * Edge provides $$FORM_ACTION encoding; browser client only has callServer RPC.
 */

const FORM_ACTION_RETURN_RE =
  /return \{\s*name: referenceClosure,\s*method: "POST",\s*encType: "multipart\/form-data",\s*data: data\s*\};/

const REGISTER_BOUND_SERVER_REFERENCE_RE =
  /function registerBoundServerReference\(reference, id, bound\) \{\s*knownServerReferences\.has\(reference\) \|\|\s*knownServerReferences\.set\(reference, \{\s*id: id,\s*originalBind: reference\.bind,\s*bound: bound\s*\}\);\s*\}/

const DEV_VALIDATED_STORE_RE =
  /Object\.defineProperty\(value\._store, "validated", \{\s*configurable: !1,\s*enumerable: !1,\s*writable: !0,\s*value: i\s*\}\);/

const DEV_VALIDATED_STORE_REPLACEMENT = `Object.defineProperty(value._store, "validated", {
                configurable: !1,
                enumerable: !1,
                writable: !0,
                value: void 0 === i ? 1 : i
              });`

const FORM_ACTION_RETURN_REPLACEMENT = `function resolveRariFormActionUrl() {
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
  };`

function replaceOnce(
  source: string,
  pattern: RegExp,
  replacement: string,
  context: string,
): string {
  const global = new RegExp(
    pattern.source,
    pattern.flags.includes('g') ? pattern.flags : `${pattern.flags}g`,
  )
  const matches = [...source.matchAll(global)]
  if (matches.length === 0) {
    throw new Error(
      `Failed to patch flight browser client: pattern for ${context} not found. ` +
        `The react-server-dom-webpack sources likely changed shape after a version bump; ` +
        `update the anchors in patch-flight-browser-client.ts.`,
    )
  }
  if (matches.length > 1) {
    throw new Error(
      `Failed to patch flight browser client: pattern for ${context} matched more than once. ` +
        `Update the anchors in patch-flight-browser-client.ts to be unambiguous.`,
    )
  }
  const match = matches[0]
  return source.slice(0, match.index) + replacement + source.slice(match.index + match[0].length)
}

function patchDevValidatedDefault(browserSource: string): string {
  if (!DEV_VALIDATED_STORE_RE.test(browserSource)) return browserSource
  DEV_VALIDATED_STORE_RE.lastIndex = 0
  return replaceOnce(
    browserSource,
    DEV_VALIDATED_STORE_RE,
    DEV_VALIDATED_STORE_REPLACEMENT,
    'DEV _store.validated default for production wire',
  )
}

function extractEdgeFormActionHelpers(edgeSource: string): string {
  const prodBoundCache = edgeSource.indexOf('var boundCache = new WeakMap();')
  const encodeFormData = edgeSource.indexOf('function encodeFormData')
  const helpersStart =
    prodBoundCache !== -1 ? prodBoundCache : encodeFormData !== -1 ? encodeFormData : -1
  const bindEnd = edgeSource.indexOf('function createBoundServerReference', helpersStart)
  if (helpersStart === -1 || bindEnd === -1) {
    throw new Error('Failed to locate edge client form-action helpers for browser patch')
  }

  let block = edgeSource.slice(helpersStart, bindEnd)
  if (!block.includes('var boundCache = new WeakMap()')) {
    block = `var boundCache = new WeakMap();\n${block}`
  }
  const hasFunctionBindDecl = /FunctionBind\s*=\s*Function\.prototype\.bind/.test(block)
  const hasArraySliceDecl = /ArraySlice\s*=\s*Array\.prototype\.slice/.test(block)
  if (!hasFunctionBindDecl && !hasArraySliceDecl) {
    block = `var FunctionBind = Function.prototype.bind,\n  ArraySlice = Array.prototype.slice;\n${block}`
  } else if (!hasFunctionBindDecl) {
    block = `var FunctionBind = Function.prototype.bind;\n${block}`
  } else if (!hasArraySliceDecl) {
    block = `var ArraySlice = Array.prototype.slice;\n${block}`
  }

  return replaceOnce(
    block,
    FORM_ACTION_RETURN_RE,
    FORM_ACTION_RETURN_REPLACEMENT,
    'edge $$FORM_ACTION return block',
  )
}

export function patchBrowserClientForFormActions(
  browserSource: string,
  edgeSource: string,
): string {
  const formActionBlock = extractEdgeFormActionHelpers(edgeSource)

  const withFormActions = replaceOnce(
    browserSource,
    REGISTER_BOUND_SERVER_REFERENCE_RE,
    formActionBlock,
    'browser registerBoundServerReference',
  )

  return patchDevValidatedDefault(withFormActions)
}
