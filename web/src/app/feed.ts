import type { Feed } from 'rari'
import { getAllBlogPosts } from '@/lib/content'

const baseUrl = 'https://rari.build'

export default function feed(): Feed {
  const posts = getAllBlogPosts()

  return {
    title: 'rari Blog',
    description:
      'Latest news, updates, and insights from the rari team. The performance-first React framework powered by Rust.',
    link: baseUrl,
    language: 'en',
    copyright: `© ${new Date().getFullYear()} rari. All rights reserved.`,
    lastBuildDate: new Date(),
    items: posts.map(post => ({
      title: post.title,
      url: `${baseUrl}/blog/${post.slug}`,
      description: post.description,
      author: post.author,
      pubDate: post.date,
      categories: ['engineering', 'react', 'rust', 'performance'],
    })),
  }
}
