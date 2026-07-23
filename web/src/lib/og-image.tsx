import { ImageResponse } from 'rari/og'
import Rari from '@/components/icons/Rari'

interface OGImageOptions {
  readonly title: string
  readonly description?: string
  readonly section?: string
  readonly logoSize?: 'small' | 'large'
}

export function generateOGImage({
  title,
  description,
  section,
  logoSize = 'small',
}: OGImageOptions) {
  const isLarge = logoSize === 'large'

  if (isLarge) {
    return new ImageResponse(
      <div
        style={{
          display: 'flex',
          width: '100%',
          height: '100%',
          background: '#0d1117',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            width: '100%',
            padding: '80px',
          }}
        >
          <Rari width={360} height={120} style={{ marginBottom: '60px' }} />

          <div
            style={{
              fontSize: 48,
              color: '#8b949e',
              marginBottom: '20px',
              textAlign: 'center',
            }}
          >
            {title}
          </div>

          <div
            style={{
              fontSize: 40,
              color: '#c9d1d9',
              textAlign: 'center',
            }}
          >
            {description}
          </div>
        </div>
      </div>,
    )
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
          background: 'linear-gradient(to bottom right, #161b22, #0d1117)',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            marginBottom: '40px',
          }}
        >
          <Rari width={120} height={40} style={{ marginRight: '20px' }} />
          {section != null && section !== '' && (
            <div
              style={{
                fontSize: 36,
                color: '#8b949e',
              }}
            >
              / {section}
            </div>
          )}
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
              marginBottom: description != null && description !== '' ? '30px' : '0',
              lineHeight: 1.2,
            }}
          >
            {title}
          </div>

          {description != null && description !== '' && (
            <div
              style={{
                fontSize: 32,
                color: '#8b949e',
                lineHeight: 1.4,
              }}
            >
              {description}
            </div>
          )}
        </div>
      </div>
    </div>,
  )
}
