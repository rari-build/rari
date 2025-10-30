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

export async function DELETE(
  _request: Request,
  { params }: { params: { id: string } },
) {
  return new Response(
    JSON.stringify({
      message: `User ${params.id} deleted`,
    }),
    {
      status: 200,
      headers: {
        'Content-Type': 'application/json',
      },
    },
  )
}
