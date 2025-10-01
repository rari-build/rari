export default function BlogPage() {
  const posts = [
    { slug: 'hello-world', title: 'Hello World', date: '2024-01-15' },
    { slug: 'app-router-guide', title: 'App Router Guide', date: '2024-01-20' },
    { slug: 'server-components', title: 'Server Components Explained', date: '2024-01-25' },
  ]

  return (
    <div style={{
      background: 'white',
      borderRadius: '12px',
      padding: '3rem',
      boxShadow: '0 10px 40px rgba(0, 0, 0, 0.1)',
    }}>
      <h1 style={{
        fontSize: '2.5rem',
        marginBottom: '2rem',
        color: '#667eea',
      }}>
        Blog Posts
      </h1>

      <div style={{
        display: 'grid',
        gap: '1.5rem',
      }}>
        {posts.map(post => (
          <a
            key={post.slug}
            href={`/blog/${post.slug}`}
            style={{
              display: 'block',
              padding: '1.5rem',
              background: '#f7fafc',
              borderRadius: '8px',
              border: '2px solid #e2e8f0',
              textDecoration: 'none',
              transition: 'all 0.2s',
            }}
          >
            <h2 style={{
              fontSize: '1.5rem',
              color: '#667eea',
              marginBottom: '0.5rem',
            }}>
              {post.title}
            </h2>
            <p style={{
              color: '#666',
              fontSize: '0.9rem',
            }}>
              Published on {post.date}
            </p>
          </a>
        ))}
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Blog | Rari App Router',
  description: 'Read our latest blog posts',
}
