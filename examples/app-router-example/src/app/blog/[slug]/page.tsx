import type { PageProps } from 'rari/client'

export default function BlogPostPage({ params }: PageProps<{ slug: string }>) {
  const { slug } = params

  const posts: Record<string, { title: string, content: string, date: string }> = {
    'hello-world': {
      title: 'Hello World',
      content: 'Welcome to our first blog post! This demonstrates dynamic routing with the app router.',
      date: '2024-01-15',
    },
    'app-router-guide': {
      title: 'App Router Guide',
      content: 'Learn how to use the app router for building modern web applications with server components.',
      date: '2024-01-20',
    },
    'server-components': {
      title: 'Server Components Explained',
      content: 'Server components allow you to render components on the server, improving performance and SEO.',
      date: '2024-01-25',
    },
  }

  const post = posts[slug] || {
    title: 'Post Not Found',
    content: 'The requested blog post could not be found.',
    date: '',
  }

  return (
    <div style={{
      background: 'white',
      borderRadius: '12px',
      padding: '3rem',
      boxShadow: '0 10px 40px rgba(0, 0, 0, 0.1)',
    }}
    >
      <a
        href="/blog"
        style={{
          color: '#667eea',
          textDecoration: 'none',
          fontSize: '0.95rem',
          marginBottom: '1rem',
          display: 'inline-block',
        }}
      >
        ← Back to Blog
      </a>

      <h1 style={{
        fontSize: '2.5rem',
        marginBottom: '0.5rem',
        color: '#667eea',
      }}
      >
        {post.title}
      </h1>

      {post.date && (
        <p style={{
          color: '#999',
          fontSize: '0.9rem',
          marginBottom: '2rem',
        }}
        >
          Published on
          {' '}
          {post.date}
        </p>
      )}

      <div style={{
        fontSize: '1.1rem',
        color: '#666',
        lineHeight: '1.8',
        marginBottom: '2rem',
      }}
      >
        {post.content}
      </div>

      <div style={{
        background: '#f7fafc',
        padding: '1.5rem',
        borderRadius: '8px',
        border: '1px solid #e2e8f0',
        marginTop: '2rem',
      }}
      >
        <h3 style={{ marginBottom: '0.5rem', color: '#667eea' }}>Route Info:</h3>
        <pre style={{ fontSize: '0.9rem', color: '#4a5568' }}>
          {JSON.stringify({ slug }, null, 2)}
        </pre>
      </div>
    </div>
  )
}

export async function generateMetadata({ params }: PageProps<{ slug: string }>) {
  const posts: Record<string, { title: string, content: string }> = {
    'hello-world': { title: 'Hello World', content: 'Welcome to our first blog post!' },
    'app-router-guide': { title: 'App Router Guide', content: 'Learn about the app router' },
    'server-components': { title: 'Server Components Explained', content: 'Understanding server components' },
  }

  const post = posts[params.slug]

  return {
    title: post ? `${post.title} | Blog` : 'Post Not Found',
    description: post?.content || 'Blog post not found',
  }
}
