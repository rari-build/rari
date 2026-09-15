export function GET(_request: Request) {
  return new Response(JSON.stringify({ message: 'Hello from API route!' }), {
    status: 200,
    headers: {
      'Content-Type': 'application/json',
    },
  })
}

export async function POST(request: Request) {
  const body: unknown = await request.json()
  return new Response(JSON.stringify({ received: body }), {
    status: 201,
    headers: {
      'Content-Type': 'application/json',
    },
  })
}
