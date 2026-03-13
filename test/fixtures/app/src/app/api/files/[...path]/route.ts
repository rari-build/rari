export async function GET(
  _request: Request,
  { params }: { params: { path: string } },
) {
  return new Response(
    JSON.stringify({
      message: 'Catch-all API route',
      path: params.path,
      segments: params.path.split('/'),
    }),
    {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    },
  )
}
