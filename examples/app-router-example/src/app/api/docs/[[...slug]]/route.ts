export async function GET(
  request: Request,
  { params }: { params: { slug?: string } },
) {
  return new Response(
    JSON.stringify({
      message: 'Optional catch-all route test',
      slug: params.slug || null,
      segments: params.slug ? params.slug.split('/') : [],
      isRoot: !params.slug,
      fullUrl: request.url,
    }),
    {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    },
  )
}
