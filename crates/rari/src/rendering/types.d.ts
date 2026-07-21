/// <reference path="../runtime/ext/rari/core/types.d.ts" />
/// <reference path="../runtime/ext/types.d.ts" />

declare global {
  const args_json: Array<unknown>
  const props_json: Record<string, unknown>
  const component_id: string
  const function_name: string
  const __RARI_ACTION_MODE__: 'form' | 'reply' | 'reply-multipart'
  const __RARI_ACTION_ID__: string
  const __RARI_ACTION_BODY__: string
  const __RARI_ACTION_BODY_B64__: string
  const __RARI_ACTION_CONTENT_TYPE__: string
  const __RARI_ACTION_FORM_ENTRIES__: Array<[string, string]>

  interface ActionValidationConfig {
    maxDepth: number
    maxStringLength: number
    maxArrayLength: number
    maxObjectKeys: number
    maxTotalElements: number
  }

  function productionValidationConfig(): ActionValidationConfig
  function developmentValidationConfig(): ActionValidationConfig
  function validateActionArgsWithConfig(args: unknown[], config: ActionValidationConfig): unknown[]
  function validateFormDataWithConfig(formData: FormData, config: ActionValidationConfig): void
  function validateActionArgs(args: unknown[]): unknown[]
  function validateFormData(formData: FormData): void
  function resolveActionFn(
    id: string,
    manifest: Record<string, { id: string, chunks: string[], name?: string }>,
  ): (...args: unknown[]) => unknown

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
        function op_fizz_chunk_try(streamId: string, text: string): number
        function op_fizz_chunk(streamId: string, text: string): Promise<void>
        function op_fizz_done(streamId: string): void
        function op_stream_promise_settled(streamId: string, ok: boolean, error: string): void
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
