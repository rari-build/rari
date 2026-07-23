export function GET(
  _request: Request,
  { params }: Readonly<{ readonly params: { readonly category: string; readonly id: string } }>,
) {
  return new Response(
    JSON.stringify({
      category: params.category,
      id: params.id,
      title: `Post in ${params.category}/${params.id}`,
    }),
    {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    },
  )
}
