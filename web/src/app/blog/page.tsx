import type { Metadata } from 'rari'
import BlogPostCard from '@/components/BlogPostCard'
import News from '@/components/icons/News'
import { getAllBlogPosts, isValidSlug } from '@/lib/content'
import { container } from '@/lib/styles'

export default function BlogIndexPage() {
  const posts = getAllBlogPosts()
  const validPosts = posts.filter(post => isValidSlug(post.slug))

  return (
    <div className={`${container.marketing} py-12`}>
      <div className="space-y-12">
        <div className="space-y-4">
          <h1 className="text-5xl font-bold text-fg tracking-tight">rari Blog</h1>
          <p className="text-xl text-fg-muted max-w-2xl">
            Latest news, updates, and insights from the rari team.
          </p>
        </div>

        {validPosts.length > 0 ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {validPosts.map(post => (
              <BlogPostCard
                key={post.slug}
                slug={post.slug}
                title={post.title}
                description={post.description}
                date={post.date}
              />
            ))}
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center py-16 px-4">
            <div className="text-center space-y-4 max-w-md">
              <div className="w-16 h-16 mx-auto bg-surface border border-edge rounded-lg flex items-center justify-center">
                <News className="w-8 h-8 text-fg-muted" />
              </div>
              <h2 className="text-2xl font-semibold text-fg">No posts yet</h2>
              <p className="text-fg-muted">
                Check back soon for updates, tutorials, and insights from the rari team.
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'rari Blog',
  description: 'Latest news and updates from the rari team.',
}
