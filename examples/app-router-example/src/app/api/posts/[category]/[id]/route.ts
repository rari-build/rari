export async function GET(
  _request: Request,
  { params }: { params: { category: string, id: string } },
) {
  return new Response(
    JSON.stringify({
      message: 'Multiple dynamic segments test',
      category: params.category,
      id: params.id,
      fullPath: `/api/posts/${params.category}/${params.id}`,
    }),
    {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    },
  )
}
