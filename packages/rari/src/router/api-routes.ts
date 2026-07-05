export interface RouteContext<TParams extends Record<string, string> = Record<string, string>> {
  params: TParams
}

export type RouteHandler<TParams extends Record<string, string> = Record<string, string>> = (
  request: Request,
  context?: RouteContext<TParams>,
) => Response | Promise<Response> | any | Promise<any>

export interface ApiRouteHandlers<TParams extends Record<string, string> = Record<string, string>> {
  GET?: RouteHandler<TParams>
  POST?: RouteHandler<TParams>
  PUT?: RouteHandler<TParams>
  DELETE?: RouteHandler<TParams>
  PATCH?: RouteHandler<TParams>
  HEAD?: RouteHandler<TParams>
  OPTIONS?: RouteHandler<TParams>
}

export class ApiResponse extends Response {
  static json(data: unknown, init?: ResponseInit): Response {
    return Response.json(data, init)
  }

  static redirect(url: string, status: number = 307): Response {
    if (/^https?:\/\//i.test(url))
      return Response.redirect(url, status)

    return new Response(null, {
      status,
      headers: { location: url },
    })
  }

  static noContent(init?: ResponseInit): Response {
    return new Response(null, {
      ...init,
      status: 204,
    })
  }
}
