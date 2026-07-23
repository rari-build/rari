import type { Metadata, PageProps } from 'rari'

export default function BlogPostPage({ params }: PageProps) {
  const { slug } = params

  return (
    <div>
      <h1>Blog Post: {slug}</h1>
      <p>This is a dynamic route with slug parameter.</p>
      <div data-testid="slug-value">{slug}</div>
      <a href="/blog">Back to Blog</a>
    </div>
  )
}

export function generateMetadata({ params }: PageProps): Metadata {
  const { slug } = params
  return {
    title: `Blog: ${String(slug)}`,
    description: `Blog post about ${String(slug)}`,
  }
}
