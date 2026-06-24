declare global {
  namespace Deno {
    namespace core {
      namespace ops {
        function op_get_cookies(): string
        function op_set_cookie(options: Record<string, unknown>): void
        function op_delete_cookie(name: string): void
      }
    }

    namespace env {
      function get(key: string): string | undefined
    }
  }
}

declare module 'ext:rari/cookies.ts' {}
declare module 'ext:rari/api_handler.ts' {}
declare module 'ext:rari/component_loader.ts' {}
declare module 'ext:rari/metadata_collector.ts' {}

export {}
