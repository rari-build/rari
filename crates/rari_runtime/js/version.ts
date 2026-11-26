// Copyright 2018-2026 the Deno authors. MIT license.

const { primordials } = __bootstrap
const { ObjectFreeze } = primordials

const version = {
  deno: '',
  v8: '',
  typescript: '',
}

function setVersions(denoVersion: string, v8Version: string, tsVersion: string): void {
  version.deno = denoVersion
  version.v8 = v8Version
  version.typescript = tsVersion

  ObjectFreeze(version)
}

export { setVersions, version }
