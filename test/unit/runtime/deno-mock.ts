interface DenoLike {
  core: {
    ops: Record<string, (...args: any[]) => any>
  }
}

export interface MockBackend {
  read: (key: string) => string | null
  write: (key: string, value: string, ttlMs: number) => void
}

export interface BackendOpNames {
  get: string
  set: string
}

let savedDeno: DenoLike | undefined
let denoWasPresent = false

function patchDeno(ops: DenoLike['core']['ops']): void {
  const target = globalThis as { Deno?: DenoLike }
  denoWasPresent = 'Deno' in target
  savedDeno = target.Deno
  target.Deno = { core: { ops } }
}

export function patchDenoBackend(opNames: BackendOpNames, backend: MockBackend): void {
  patchDeno({
    [opNames.get]: async (key: string) => backend.read(key),
    [opNames.set]: async (key: string, value: string, ttlMs: number) =>
      backend.write(key, value, ttlMs),
  })
}

export function restoreDeno(): void {
  const target = globalThis as { Deno?: DenoLike }
  if (denoWasPresent) {
    target.Deno = savedDeno
  }
  else {
    delete target.Deno
  }
  savedDeno = undefined
  denoWasPresent = false
}
