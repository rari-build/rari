export function GET(_request: Request) {
  return new Response(JSON.stringify({ message: 'Hello from API!' }), {
    status: 200,
    headers: {
      'Content-Type': 'application/json',
    },
  })
}

export async function POST(request: Request) {
  const body: unknown = await request.json()
  return new Response(JSON.stringify({ received: body, echo: true }), {
    status: 201,
    headers: {
      'Content-Type': 'application/json',
    },
  })
}

export async function PUT(request: Request) {
  const body: unknown = await request.json()
  return new Response(JSON.stringify({ updated: body }), {
    status: 200,
    headers: {
      'Content-Type': 'application/json',
    },
  })
}

export function DELETE(_request: Request) {
  return new Response(JSON.stringify({ deleted: true }), {
    status: 200,
    headers: {
      'Content-Type': 'application/json',
    },
  })
}

export async function PATCH(request: Request) {
  const body: unknown = await request.json()
  return new Response(JSON.stringify({ patched: body }), {
    status: 200,
    headers: {
      'Content-Type': 'application/json',
    },
  })
}
