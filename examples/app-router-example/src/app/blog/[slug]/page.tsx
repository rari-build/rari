import type { PageProps } from 'rari'
import { getBlogPost } from '@/data/blog-posts'

export default function BlogPostPage({ params }: PageProps<{ slug: string }>) {
  const { slug } = params

  const post = getBlogPost(slug) || {
    title: 'Post Not Found',
    content: 'The requested blog post could not be found.',
    date: '',
  }

  return (
    <div className="bg-white rounded-xl p-12 shadow-2xl">
      <a
        href="/blog"
        className="text-indigo-600 no-underline text-sm mb-4 inline-block hover:text-indigo-700 transition-colors"
      >
        ← Back to Blog
      </a>

      <h1 className="text-4xl mb-2 text-gray-900 font-bold">{post.title}</h1>

      {post.date && (
        <p className="text-gray-400 text-sm mb-8">
          Published on
          {post.date}
        </p>
      )}

      <div className="text-lg text-gray-600 leading-loose mb-8">
        {post.content}
      </div>

      <div className="bg-gray-50 p-6 rounded-lg border border-gray-200 mt-8">
        <h3 className="mb-2 text-gray-900 font-semibold">Route Info:</h3>
        <pre className="text-sm text-gray-700">
          {JSON.stringify({ slug }, null, 2)}
        </pre>
      </div>
    </div>
  )
}

export async function generateMetadata({
  params,
}: PageProps<{ slug: string }>) {
  const post = getBlogPost(params.slug)

  return {
    title: post ? `${post.title} | Blog` : 'Post Not Found',
    description: post?.content || 'Blog post not found',
  }
}
