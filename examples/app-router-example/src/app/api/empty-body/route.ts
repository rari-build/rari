export async function POST(request: Request) {
  try {
    const body = await request.text()

    if (!body) {
      return new Response(
        JSON.stringify({
          message: 'Empty body received',
          bodyLength: 0,
        }),
        {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        },
      )
    }

    const parsed = JSON.parse(body)
    return new Response(
      JSON.stringify({
        message: 'Body received',
        body: parsed,
      }),
      {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      },
    )
  }
  catch (error) {
    return new Response(
      JSON.stringify({
        error: 'Invalid JSON',
        message: error instanceof Error ? error.message : 'Unknown error',
      }),
      {
        status: 400,
        headers: { 'Content-Type': 'application/json' },
      },
    )
  }
}
