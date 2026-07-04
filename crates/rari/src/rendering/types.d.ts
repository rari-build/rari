/// <reference path="../runtime/ext/rari/core/types.d.ts" />
/// <reference path="../runtime/ext/types.d.ts" />

declare global {
  const args_json: Array<unknown>
  const props_json: Record<string, unknown>
  const component_id: string
  const function_name: string
  const composition_script: Promise<{
    rsc_data: string
    boundaries?: Array<{
      id: string
      fallback: unknown
      parentId?: string
      parentPath?: string[]
      isInContentArea?: boolean
    }>
    pending_promises?: unknown[]
  }>

  namespace Deno {
    namespace core {
      namespace ops {
        function op_sanitize_html(html: string, componentId: string): string
        function op_fizz_chunk(text: string): Promise<void>
        function op_fizz_done(): void
        function op_internal_log(message: string): void
      }
    }
  }

  interface LazyPromiseEntry {
    isDeferred?: boolean
    component?: (props: unknown) => Promise<unknown>
    props?: unknown
    promise?: Promise<unknown>
  }

  interface LazyPromiseResult {
    success: boolean
    data?: unknown
    error?: string
    stack?: string
  }
}

export {}
