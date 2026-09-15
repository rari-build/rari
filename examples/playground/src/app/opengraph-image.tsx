import { ImageResponse } from 'rari/og'

export default function Image() {
  return new ImageResponse(
    <div
      style={{
        display: 'flex',
        width: '100%',
        height: '100%',
        background: 'linear-gradient(to right, #667eea, #764ba2)',
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
              fontSize: 180,
              fontWeight: 'bold',
              color: 'white',
            }}
          >
            rari
          </div>
          <div
            style={{
              fontSize: 140,
              marginLeft: '40px',
            }}
          >
            🚀
          </div>
        </div>

        <div
          style={{
            fontSize: 56,
            color: 'white',
            marginBottom: '30px',
          }}
        >
          The Fast React Framework
        </div>

        <div
          style={{
            fontSize: 40,
            color: 'white',
          }}
        >
          Server-First • Type-Safe • Blazing-fast ⚡
        </div>
      </div>
    </div>,
  )
}
