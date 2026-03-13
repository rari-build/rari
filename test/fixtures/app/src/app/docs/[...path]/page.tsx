import type { Metadata, PageProps } from 'rari'

export default async function DocsPage({ params }: PageProps) {
  const { path } = params
  const pathArray = Array.isArray(path) ? path : [path]
  const pathString = pathArray.join('/')

  return (
    <div>
      <h1>
        Docs:
        {' '}
        {pathString}
      </h1>
      <p>This is a catch-all route.</p>
      <div data-testid="path-segments" data-segments={JSON.stringify(pathArray)}>
        {pathArray.map((segment, i) => (
          <span key={i} data-testid={`segment-${i}`}>{segment}</span>
        ))}
      </div>
      <div data-testid="path-length">{String(pathArray.length)}</div>
    </div>
  )
}

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { path } = params
  const pathArray = Array.isArray(path) ? path : [path]
  const pathString = pathArray.join('/')

  return {
    title: `Docs: ${pathString}`,
    description: `Documentation for ${pathString}`,
  }
}
