import { DefaultLoading } from 'rari/client'

export default function Loading() {
  return (
    <div className="bg-white rounded-xl p-12 shadow-2xl min-h-[300px] flex items-center justify-center">
      <DefaultLoading />
    </div>
  )
}
