import type { PageProps } from 'rari'
import { ImageResponse } from 'rari/og'
import { getBlogPost } from '@/data/blog-posts'

export default function Image({ params }: PageProps<{ slug: string }>) {
  const post = getBlogPost(params.slug) || {
    title: 'Post Not Found',
    content: 'The requested blog post could not be found.',
    date: '',
  }

  return new ImageResponse(
    <div
      style={{
        display: 'flex',
        width: '100%',
        height: '100%',
        padding: '20px',
        gap: '20px',
        background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        color: 'white',
      }}
    >
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          width: '60%',
          padding: '40px 60px',
          gap: '24px',
          justifyContent: 'space-between',
          border: '4px solid white',
          borderRadius: '16px',
        }}
      >
        <div
          style={{
            fontSize: 40,
            fontWeight: 'bold',
            textDecoration: 'underline',
          }}
        >
          Rari Blog
        </div>

        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '16px',
          }}
        >
          <div
            style={{
              fontSize: 64,
              fontWeight: 'bold',
              lineHeight: 1.1,
            }}
          >
            {post.title}
          </div>
          <div
            style={{
              fontSize: 28,
              lineHeight: 1.4,
            }}
          >
            {post.content}
          </div>
        </div>

        {post.date && (
          <div
            style={{
              fontSize: 24,
              textAlign: 'right',
            }}
          >
            {post.date}
          </div>
        )}
      </div>

      <div
        style={{
          display: 'flex',
          width: '40%',
        }}
      >
        <img
          src="https://images.unsplash.com/photo-1555066931-4365d14bab8c?w=400&h=500&fit=crop"
          style={{
            width: '100%',
            height: '100%',
            objectFit: 'cover',
            borderRadius: '12px',
          }}
        />
      </div>
    </div>,
  )
}
