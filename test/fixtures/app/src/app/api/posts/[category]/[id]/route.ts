export async function GET(
  _request: Request,
  { params }: { params: { category: string, id: string } },
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
