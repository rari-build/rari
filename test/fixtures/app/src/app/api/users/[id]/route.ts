export async function GET(
  _request: Request,
  { params }: { params: { id: string } },
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
  { params }: { params: { id: string } },
) {
  const body = await request.json()
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

export async function DELETE(
  _request: Request,
  { params }: { params: { id: string } },
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
