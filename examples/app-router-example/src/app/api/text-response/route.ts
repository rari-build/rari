export async function GET(_request: Request) {
  return new Response('This is plain text', {
    status: 200,
    headers: { 'Content-Type': 'text/plain' },
  })
}

export async function POST(_request: Request) {
  return new Response('<html><body><h1>HTML Response</h1></body></html>', {
    status: 200,
    headers: { 'Content-Type': 'text/html' },
  })
}
