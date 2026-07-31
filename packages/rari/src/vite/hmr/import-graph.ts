/**
 * Collect every file that transitively imports one of the seed files, walking
 * the reverse import graph (file -> set of files importing it). Seeds are
 * traversed but excluded from the result; pass a predicate to keep only a
 * subset of the discovered importers.
 */
export function walkImporters(
  importGraph: ReadonlyMap<string, ReadonlySet<string>>,
  seeds: readonly string[],
  predicate?: (file: string) => boolean,
): Set<string> {
  const visited = new Set(seeds)
  const collected = new Set<string>()
  const queue = [...seeds]

  while (queue.length > 0) {
    const current = queue.pop()
    if (current == null) break

    const importers = importGraph.get(current)
    if (!importers) continue

    for (const importer of importers) {
      if (visited.has(importer)) continue
      visited.add(importer)
      queue.push(importer)

      if (predicate == null || predicate(importer)) collected.add(importer)
    }
  }

  return collected
}
