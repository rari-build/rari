export async function GET(request: Request) {
  const url = new URL(request.url)
  const errorType = url.searchParams.get('type')

  if (errorType === 'throw')
    throw new Error('Intentional error for testing')

  if (errorType === 'invalid-json') {
    return new Response('This is not JSON', {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    })
  }

  return new Response(
    JSON.stringify({
      message: 'Error test endpoint',
      hint: 'Try ?type=throw or ?type=invalid-json',
    }),
    {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    },
  )
}
