import { ImageResponse } from 'rari/og'

export default function Image() {
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
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            marginBottom: '60px',
          }}
        >
          <div
            style={{
              width: '100px',
              height: '100px',
              background: 'linear-gradient(to bottom right, #fd7e14, #e8590c)',
              borderRadius: '24px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              marginRight: '30px',
            }}
          >
            <span
              style={{
                fontSize: 60,
                fontWeight: 'bold',
                color: 'white',
              }}
            >
              R
            </span>
          </div>
          <div
            style={{
              fontSize: 120,
              fontWeight: 'bold',
              color: '#f0f6fc',
            }}
          >
            rari
          </div>
        </div>

        <div
          style={{
            fontSize: 72,
            color: '#fd7e14',
            fontWeight: 'bold',
            marginBottom: '30px',
            textAlign: 'center',
          }}
        >
          Blog
        </div>

        <div
          style={{
            fontSize: 40,
            color: '#8b949e',
            textAlign: 'center',
          }}
        >
          Latest news, updates, and insights
        </div>
      </div>
    </div>,
  )
}
