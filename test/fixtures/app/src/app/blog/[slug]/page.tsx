import type { Metadata, PageProps } from 'rari'

export default async function BlogPostPage({ params }: PageProps) {
  const { slug } = params

  return (
    <div>
      <h1>
        Blog Post:
        {' '}
        {slug}
      </h1>
      <p>This is a dynamic route with slug parameter.</p>
      <div data-testid="slug-value">{slug}</div>
      <a href="/blog">Back to Blog</a>
    </div>
  )
}

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { slug } = params
  return {
    title: `Blog: ${slug}`,
    description: `Blog post about ${slug}`,
  }
}
