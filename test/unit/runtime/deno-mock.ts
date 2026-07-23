interface DenoLike {
  core: {
    ops: Record<string, (...args: readonly any[]) => any>
  }
}

export interface MockBackend {
  readonly read: (key: string) => string | null
  readonly write: (key: string, value: string, ttlMs: number) => void
}

export interface BackendOpNames {
  readonly get: string
  readonly set: string
}

let savedDeno: DenoLike | undefined
let denoWasPresent = false

function patchDeno(
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- Deno ops bag is patched in place
  ops: DenoLike['core']['ops'],
): void {
  const target = globalThis as { Deno?: DenoLike }
  denoWasPresent = 'Deno' in target
  savedDeno = target.Deno
  target.Deno = { core: { ops } }
}

export function patchDenoBackend(
  opNames: BackendOpNames,
  backend: MockBackend,
  options?: Readonly<{ remoteHandler?: 'redis' | 'redb' | 'test' }>,
): void {
  const ops: DenoLike['core']['ops'] = {
    [opNames.get]: (key: string) => backend.read(key),
    [opNames.set]: (key: string, value: string, ttlMs: number) => {
      backend.write(key, value, ttlMs)
    },
  }
  if (options?.remoteHandler) ops.op_use_cache_remote_handler = () => options.remoteHandler
  patchDeno(ops)
}

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- Deno ops bag is patched in place
export function patchDenoOps(ops: DenoLike['core']['ops']): void {
  patchDeno(ops)
}

export function restoreDeno(): void {
  const target = globalThis as { Deno?: DenoLike }
  if (denoWasPresent) target.Deno = savedDeno
  else delete target.Deno
  savedDeno = undefined
  denoWasPresent = false
}
