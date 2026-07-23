import process from 'node:process'

const ECHO_URL = `http://localhost:${process.env.PORT != null && process.env.PORT !== '' ? process.env.PORT : 3000}/test-fetch.json`

interface EchoPayload {
  ok: boolean
  message: string
  counter: number
}

async function loadEcho(): Promise<EchoPayload> {
  const res = await fetch(ECHO_URL, {
    next: { revalidate: 60, tags: ['echo'] },
  } as RequestInit & { next?: { revalidate?: number; tags?: string[] } })
  const data: unknown = await res.json()
  if (
    typeof data !== 'object' ||
    data === null ||
    !('ok' in data) ||
    typeof data.ok !== 'boolean' ||
    !('message' in data) ||
    typeof data.message !== 'string' ||
    !('counter' in data) ||
    typeof data.counter !== 'number'
  ) {
    throw new Error('Invalid echo payload')
  }

  return {
    ok: data.ok,
    message: data.message,
    counter: data.counter,
  }
}

export default async function FetchTestPage() {
  const data = await loadEcho()
  return (
    <div>
      <h1>Fetch Cache Test</h1>
      <p data-testid="echo-ok">{data.ok ? 'true' : 'false'}</p>
      <p data-testid="echo-message">{data.message}</p>
      <p data-testid="echo-counter">
        counter=
        {data.counter}
      </p>
    </div>
  )
}
