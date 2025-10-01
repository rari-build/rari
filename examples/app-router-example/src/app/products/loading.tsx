import { DefaultLoading } from 'rari/client'

export default function Loading() {
  return (
    <div style={{
      background: 'white',
      borderRadius: '12px',
      padding: '3rem',
      boxShadow: '0 10px 40px rgba(0, 0, 0, 0.1)',
      minHeight: '300px',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
    }}
    >
      <DefaultLoading />
    </div>
  )
}
