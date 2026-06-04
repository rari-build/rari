import type { Metadata } from 'rari'

let callCount = 0

async function getCachedData(label: string) {
  'use cache'
  callCount++
  return `${label}:${callCount}`
}

export default async function UseCachePage() {
  const result1 = await getCachedData('first')
  const result2 = await getCachedData('first')
  const result3 = await getCachedData('second')

  return (
    <div>
      <h1>use cache Test</h1>
      <p data-testid="result1">{result1}</p>
      <p data-testid="result2">{result2}</p>
      <p data-testid="result3">{result3}</p>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'use cache Test',
}
