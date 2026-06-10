import type { Metadata } from 'rari'

let callCount = 0
let renderCount = 0
const callLog: string[] = []

async function getCachedData(label: string) {
  'use cache'
  callCount++
  return `${label}:${callCount}`
}

export default async function UseCachePage() {
  callCount = 0
  callLog.length = 0
  renderCount++

  const result1 = await getCachedData('first')
  callLog.push(result1)
  const result2 = await getCachedData('first')
  callLog.push(result2)
  const result3 = await getCachedData('second')
  callLog.push(result3)

  return (
    <div>
      <h1>use cache Test</h1>
      <p data-testid="result1">{result1}</p>
      <p data-testid="result2">{result2}</p>
      <p data-testid="result3">{result3}</p>
      <meta data-render-count={renderCount} data-call-log={callLog.join(',')} />
    </div>
  )
}

export const metadata: Metadata = {
  title: 'use cache Test',
}
