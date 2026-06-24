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

export {}
