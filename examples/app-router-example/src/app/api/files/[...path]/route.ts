export async function GET(
  request: Request,
  { params }: { params: { path: string } },
) {
  return new Response(
    JSON.stringify({
      message: 'Catch-all route test',
      path: params.path,
      segments: params.path.split('/'),
      fullUrl: request.url,
    }),
    {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    },
  )
}
