export async function GET(request: Request) {
  const url = new URL(request.url)
  const page = url.searchParams.get('page') || '1'
  const limit = url.searchParams.get('limit') || '10'

  return new Response(
    JSON.stringify({
      message: 'Query parameters test',
      page,
      limit,
      allParams: Object.fromEntries(url.searchParams),
    }),
    {
      status: 200,
      headers: {
        'Content-Type': 'application/json',
        'X-Custom-Header': 'test-value',
      },
    },
  )
}

export async function PUT(request: Request) {
  const body = await request.json()
  return new Response(
    JSON.stringify({
      message: 'PUT method test',
      received: body,
    }),
    {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    },
  )
}

export async function PATCH(request: Request) {
  const body = await request.json()
  return new Response(
    JSON.stringify({
      message: 'PATCH method test',
      received: body,
    }),
    {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    },
  )
}
