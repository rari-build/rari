import { ImageResponse } from 'rari/og'
import Rari from '@/components/icons/Rari'

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
        <Rari
          width={360}
          height={120}
          style={{ marginBottom: '60px' }}
        />

        <div
          style={{
            fontSize: 48,
            color: '#8b949e',
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
            textAlign: 'center',
          }}
        >
          Performance-first React framework powered by Rust
        </div>
      </div>
    </div>,
  )
}
