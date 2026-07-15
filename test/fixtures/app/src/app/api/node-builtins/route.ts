import { NODE_BUILTIN_MODULES } from '../../../lib/node-builtins'

export async function GET(request: Request) {
  const name = new URL(request.url).searchParams.get('name')

  if (!name) {
    return Response.json({
      modules: [...NODE_BUILTIN_MODULES],
      total: NODE_BUILTIN_MODULES.length,
    })
  }

  if (!(NODE_BUILTIN_MODULES as readonly string[]).includes(name)) {
    return Response.json({ name, ok: false, error: 'unknown builtin' }, { status: 400 })
  }

  try {
    await import(`node:${name}`)
    return Response.json({ name, ok: true })
  }
  catch (error) {
    return Response.json({
      name,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
