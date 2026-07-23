export function GET(
  request: Request,
  { params }: Readonly<{ readonly params: { readonly slug?: string } }>,
) {
  return new Response(
    JSON.stringify({
      message: 'Optional catch-all route test',
      slug: params.slug != null && params.slug !== '' ? params.slug : null,
      segments: params.slug != null && params.slug !== '' ? params.slug.split('/') : [],
      isRoot: params.slug == null || params.slug === '',
      fullUrl: request.url,
    }),
    {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    },
  )
}
