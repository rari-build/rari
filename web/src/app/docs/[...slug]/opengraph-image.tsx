import type { PageProps } from 'rari'
import { readFile } from 'node:fs/promises'
import { ImageResponse } from 'rari/og'
import { getDocsFilePath, isValidSlugArray } from '@/lib/content-utils'

export default async function Image({ params }: PageProps) {
  const slug = params?.slug
  let title = 'Rari Docs'
  let description = 'Complete documentation for Rari framework.'

  if (isValidSlugArray(slug)) {
    try {
      const content = await readFile(getDocsFilePath(slug), 'utf-8')
      const titleMatch = content.match(/^export\s+const\s+title\s*=\s*['"](.+)['"]/m)
      const descriptionMatch = content.match(/^export\s+const\s+description\s*=\s*['"](.+)['"]/m)

      if (titleMatch)
        title = titleMatch[1]
      if (descriptionMatch)
        description = descriptionMatch[1]
    }
    catch {}
  }

  return new ImageResponse(
    <div
      style={{
        display: 'flex',
        width: '100%',
        height: '100%',
        background: '#0d1117',
        padding: '80px',
      }}
    >
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          width: '100%',
          height: '100%',
          border: '2px solid #30363d',
          borderRadius: '24px',
          padding: '60px',
          background: '#161b22',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            marginBottom: '40px',
          }}
        >
          <div
            style={{
              width: '60px',
              height: '60px',
              background: 'linear-gradient(to bottom right, #fd7e14, #e8590c)',
              borderRadius: '12px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              marginRight: '20px',
            }}
          >
            <span
              style={{
                fontSize: 36,
                fontWeight: 'bold',
                color: 'white',
              }}
            >
              R
            </span>
          </div>
          <div
            style={{
              fontSize: 48,
              fontWeight: 'bold',
              color: '#f0f6fc',
              fontFamily: 'monospace',
            }}
          >
            rari
          </div>
          <div
            style={{
              fontSize: 36,
              color: '#8b949e',
              marginLeft: '20px',
            }}
          >
            / docs
          </div>
        </div>

        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            flex: 1,
            justifyContent: 'center',
          }}
        >
          <div
            style={{
              fontSize: 56,
              fontWeight: 'bold',
              color: '#f0f6fc',
              marginBottom: '30px',
              lineHeight: 1.2,
              display: '-webkit-box',
              WebkitLineClamp: 3,
              WebkitBoxOrient: 'vertical',
              overflow: 'hidden',
            }}
          >
            {title}
          </div>

          <div
            style={{
              fontSize: 32,
              color: '#8b949e',
              lineHeight: 1.4,
              display: '-webkit-box',
              WebkitLineClamp: 2,
              WebkitBoxOrient: 'vertical',
              overflow: 'hidden',
            }}
          >
            {description}
          </div>
        </div>
      </div>
    </div>,
  )
}
