export async function GET(
  request: Request,
  { params }: { params: { slug?: string } },
) {
  const url = new URL(request.url)
  const format = url.searchParams.get('format')

  return new Response(
    JSON.stringify({
      message: 'Optional catch-all API route',
      slug: params.slug || null,
      segments: params.slug ? params.slug.split('/') : [],
      isRoot: !params.slug,
      format: format || 'default',
    }),
    {
      status: 200,
      headers: {
        'Content-Type': 'application/json',
      },
    },
  )
}
