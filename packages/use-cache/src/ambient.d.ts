declare module 'react-server-dom-webpack/client' {
  export function createTemporaryReferenceSet(): Map<string, unknown>

  export function encodeReply(
    value: unknown,
    options?: Readonly<{
      temporaryReferences?: Map<string, unknown>
      signal?: AbortSignal
    }>,
  ): Promise<FormData | string>
}
