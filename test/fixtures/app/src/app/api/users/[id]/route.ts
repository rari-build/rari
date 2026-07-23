export function GET(
  _request: Request,
  { params }: Readonly<{ readonly params: { readonly id: string } }>,
) {
  return new Response(
    JSON.stringify({
      id: params.id,
      name: `User ${params.id}`,
      email: `user${params.id}@example.com`,
    }),
    {
      status: 200,
      headers: {
        'Content-Type': 'application/json',
      },
    },
  )
}

export async function PUT(
  request: Request,
  { params }: Readonly<{ readonly params: { readonly id: string } }>,
) {
  const body: unknown = await request.json()
  if (typeof body !== 'object' || body === null) {
    return new Response(JSON.stringify({ error: 'Invalid JSON body' }), {
      status: 400,
      headers: { 'Content-Type': 'application/json' },
    })
  }

  return new Response(
    JSON.stringify({
      id: params.id,
      ...body,
      updated: true,
    }),
    {
      status: 200,
      headers: {
        'Content-Type': 'application/json',
      },
    },
  )
}

export function DELETE(
  _request: Request,
  { params }: Readonly<{ readonly params: { readonly id: string } }>,
) {
  return new Response(
    JSON.stringify({
      message: `User ${params.id} deleted`,
      id: params.id,
    }),
    {
      status: 200,
      headers: {
        'Content-Type': 'application/json',
      },
    },
  )
}
