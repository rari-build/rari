interface BlogPost {
  slug: string
  title: string
  content: string
  date: string
}

const blogPosts: Record<string, BlogPost> = {
  'hello-world': {
    slug: 'hello-world',
    title: 'Hello World',
    content:
      'Welcome to our first blog post! This demonstrates dynamic routing with the app router.',
    date: '2025-01-15',
  },
  'app-router-guide': {
    slug: 'app-router-guide',
    title: 'App Router Guide',
    content:
      'Learn how to use the app router for building modern web applications with server components.',
    date: '2025-01-20',
  },
  'server-components': {
    slug: 'server-components',
    title: 'Server Components Explained',
    content:
      'Server components allow you to render components on the server, improving performance and SEO.',
    date: '2025-01-25',
  },
}

export function getBlogPost(slug: string): BlogPost | null {
  return blogPosts[slug] || null
}

export function getAllBlogPosts(): BlogPost[] {
  return Object.values(blogPosts)
}
