import { getAllBlogPosts } from '@/data/blog-posts'

export default async function BlogPage() {
  const posts = getAllBlogPosts()

  return (
    <div className="bg-white rounded-xl p-12 shadow-2xl">
      <h1 className="text-4xl font-bold mb-8 text-gray-900">Blog Posts</h1>

      <div className="grid gap-6">
        {posts.map(post => (
          <a
            key={post.slug}
            href={`/blog/${post.slug}`}
            className="block p-6 bg-gray-50 rounded-lg border border-gray-200 no-underline hover:border-gray-300 hover:shadow-md transition-all"
          >
            <h2 className="text-2xl text-gray-900 mb-2 font-semibold">
              {post.title}
            </h2>
            <p className="text-gray-600 text-sm">
              Published on
              {post.date}
            </p>
          </a>
        ))}
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Blog | rari App Router',
  description: 'Read our latest blog posts',
}
