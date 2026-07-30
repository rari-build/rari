import process from 'node:process'

const DEFAULT_SERVER_PORT = 3000

export function getRariServerPort(): number {
  const { SERVER_PORT, PORT, RSC_PORT } = process.env
  if (SERVER_PORT != null && SERVER_PORT !== '') return Number(SERVER_PORT)
  if (PORT != null && PORT !== '') return Number(PORT)
  if (RSC_PORT != null && RSC_PORT !== '') return Number(RSC_PORT)
  return DEFAULT_SERVER_PORT
}

export function getRariServerUrl(): string {
  return `http://localhost:${getRariServerPort()}`
}
