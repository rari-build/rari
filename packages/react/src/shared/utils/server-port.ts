import process from 'node:process'

const DEFAULT_SERVER_PORT = 3000

function parsePort(value: string | undefined): number | null {
  if (value == null || value === '') return null
  const port = Number(value)
  return Number.isFinite(port) ? port : null
}

/**
 * Resolves the Rust server port from the environment.
 * Precedence: SERVER_PORT > PORT > RSC_PORT > 3000.
 * Non-numeric values are skipped in favor of the next candidate.
 */
export function getRariServerPort(): number {
  const { SERVER_PORT, PORT, RSC_PORT } = process.env
  return parsePort(SERVER_PORT) ?? parsePort(PORT) ?? parsePort(RSC_PORT) ?? DEFAULT_SERVER_PORT
}

export function getRariServerUrl(): string {
  return `http://localhost:${getRariServerPort()}`
}
