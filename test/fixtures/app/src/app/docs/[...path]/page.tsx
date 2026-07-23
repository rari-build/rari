import type { Metadata, PageProps } from 'rari'

function normalizePathSegments(path: string | readonly string[] | undefined): string[] {
  if (path == null) return []
  if (typeof path === 'string') return [path]

  return [...path]
}

export default function DocsPage({ params }: PageProps) {
  const pathArray = normalizePathSegments(params.path)
  const pathString = pathArray.join('/')

  return (
    <div>
      <h1>Docs: {pathString}</h1>
      <p>This is a catch-all route.</p>
      <div data-testid="path-segments" data-segments={JSON.stringify(pathArray)}>
        {pathArray.map((segment, i) => (
          <span key={segment} data-testid={`segment-${i}`}>
            {segment}
          </span>
        ))}
      </div>
      <div data-testid="path-length">{String(pathArray.length)}</div>
    </div>
  )
}

export function generateMetadata({ params }: PageProps): Metadata {
  const pathArray = normalizePathSegments(params.path)
  const pathString = pathArray.join('/')

  return {
    title: `Docs: ${pathString}`,
    description: `Documentation for ${pathString}`,
  }
}
