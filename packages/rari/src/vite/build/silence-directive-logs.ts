type DirectiveLog = Readonly<{
  code?: string
  message?: string
}>

export function isReactDirectiveLog(log: DirectiveLog): boolean {
  return (
    log.code === 'MODULE_LEVEL_DIRECTIVE' &&
    (log.message?.includes('use client') === true || log.message?.includes('use server') === true)
  )
}

export function createSilenceReactDirectiveLogsPlugin(): {
  name: string
  onLog: (_level: string, log: DirectiveLog) => false | undefined
} {
  return {
    name: 'rari:silence-react-directive-logs',
    onLog(_level, log) {
      return isReactDirectiveLog(log) ? false : undefined
    },
  }
}
