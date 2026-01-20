import { readdir, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import process from 'node:process'
import News from '@/components/icons/News'

interface BlogPost {
  slug: string
  title: string
  description: string
  date: string
  author?: string
}

async function getBlogPosts(): Promise<BlogPost[]> {
  try {
    const blogDir = join(process.cwd(), 'public', 'content', 'blog')
    const files = await readdir(blogDir)
    const mdxFiles = files.filter(file => file.endsWith('.mdx'))

    const posts = await Promise.all(
      mdxFiles.map(async (file) => {
        const slug = file.replace('.mdx', '')
        const content = await readFile(join(blogDir, file), 'utf-8')

        const titleMatch = content.match(/^export\s+const\s+title\s*=\s*['"](.+)['"]/m)
        const descriptionMatch = content.match(/^export\s+const\s+description\s*=\s*['"](.+)['"]/m)
        const dateMatch = content.match(/^export\s+const\s+date\s*=\s*['"](.+)['"]/m)
        const authorMatch = content.match(/^export\s+const\s+author\s*=\s*['"](.+)['"]/m)

        return {
          slug,
          title: titleMatch?.[1] || 'Untitled',
          description: descriptionMatch?.[1] || '',
          date: dateMatch?.[1] || '',
          author: authorMatch?.[1],
        }
      }),
    )

    return posts.sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime())
  }
  catch {
    return []
  }
}

function formatDate(dateString: string): string {
  if (!dateString)
    return ''
  const date = new Date(dateString)
  return date.toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' })
}

export default async function BlogIndexPage() {
  const posts = await getBlogPosts()

  return (
    <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full">
      <div className="space-y-12">
        <div className="space-y-4">
          <h1 className="text-5xl font-bold text-[#f0f6fc] tracking-tight">
            rari Blog
          </h1>
          <p className="text-xl text-gray-300 max-w-2xl">
            Latest news, updates, and insights from the rari team.
          </p>
        </div>

        {posts.length > 0
          ? (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {posts.map(post => (
                  <a
                    key={post.slug}
                    href={`/blog/${post.slug}`}
                    className="group block p-6 bg-[#161b22] border border-[#30363d] rounded-lg hover:border-[#fd7e14] hover:shadow-lg hover:shadow-[#fd7e14]/10 transition-all duration-200"
                  >
                    <div className="flex items-center gap-2 text-sm text-gray-500 mb-3">
                      <time>{formatDate(post.date)}</time>
                    </div>
                    <h2 className="text-xl font-semibold text-[#f0f6fc] mb-3 group-hover:text-[#fd7e14] transition-colors">
                      {post.title}
                    </h2>
                    <p className="text-gray-300 leading-relaxed">
                      {post.description}
                    </p>
                  </a>
                ))}
              </div>
            )
          : (
              <div className="flex flex-col items-center justify-center py-16 px-4">
                <div className="text-center space-y-4 max-w-md">
                  <div className="w-16 h-16 mx-auto bg-[#161b22] border border-[#30363d] rounded-lg flex items-center justify-center">
                    <News className="w-8 h-8 text-gray-500" />
                  </div>
                  <h2 className="text-2xl font-semibold text-[#f0f6fc]">No posts yet</h2>
                  <p className="text-gray-300">
                    Check back soon for updates, tutorials, and insights from the rari team.
                  </p>
                </div>
              </div>
            )}
      </div>
    </div>
  )
}

export const metadata = {
  title: 'rari Blog',
  description: 'Latest news and updates from the rari team.',
}
