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
              width: '120px',
              height: '120px',
              background: 'linear-gradient(to bottom right, #fd7e14, #e8590c)',
              borderRadius: '32px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              marginRight: '40px',
            }}
          >
            <span
              style={{
                fontSize: 72,
                fontWeight: 'bold',
                color: 'white',
              }}
            >
              R
            </span>
          </div>
          <div
            style={{
              fontSize: 140,
              fontWeight: 'bold',
              color: '#f0f6fc',
              fontFamily: 'monospace',
            }}
          >
            rari
          </div>
        </div>

        <div
          style={{
            fontSize: 48,
            color: '#8b949e',
            opacity: 0.95,
            marginBottom: '20px',
            textAlign: 'center',
          }}
        >
          Runtime Accelerated Rendering Infrastructure
        </div>

        <div
          style={{
            fontSize: 40,
            color: '#c9d1d9',
            opacity: 0.9,
            textAlign: 'center',
          }}
        >
          Performance-first React framework powered by Rust
        </div>
      </div>
    </div>,
  )
}
