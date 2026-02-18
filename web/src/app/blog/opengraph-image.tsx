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
          width={279}
          height={93}
          style={{ marginBottom: '60px' }}
        />

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
