export function buildCacheKeyArgs(args: readonly unknown[], argCount: number): readonly unknown[] {
  if (args.length <= argCount) return args.slice(-argCount)

  const bound = args[0]
  if (!Array.isArray(bound)) return args.slice(-argCount)

  const boundArgs = bound as readonly unknown[]
  return [...boundArgs.slice(1), ...args.slice(1)]
}
