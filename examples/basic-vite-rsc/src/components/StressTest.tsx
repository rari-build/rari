interface StressTestProps {
  itemCount?: number
}

export default async function StressTest({ itemCount = 50000 }: StressTestProps) {
  // More precise timing using high-resolution performance measurement
  const startTime = performance.now()

  // Nuclear computation - massive math operations
  const numbers = []
  for (let i = 0; i < itemCount; i++) {
    numbers.push({
      id: i,
      value: i * 2,
      squared: i * i,
      cubed: i * i * i,
      factorial: i < 10 ? Array.from({ length: i }, (_, idx) => idx + 1).reduce((acc, val) => acc * val, 1) : 0,
      isPrime: i > 1 ? Array.from({ length: Math.sqrt(i) }, (_, idx) => idx + 2).every(divisor => i % divisor !== 0) : false,
      category: i % 8,
      nested: {
        deep: {
          calculations: Array.from({ length: 5 }, (_, idx) => i * idx * Math.PI),
          metadata: `item-${i}-processed`,
          score: Math.sin(i) * Math.cos(i) * 100,
        },
      },
    })
  }

  // Nuclear data processing - multiple complex operations
  const processed = numbers
    .filter(n => n.value > 10 && n.squared % 3 === 0)
    .map(n => ({
      ...n,
      computedScore: n.nested.deep.score * n.value / 100,
      complexCalc: n.cubed + n.squared - n.value,
      trigResult: Math.tan(n.id) * 1000,
    }))
    .sort((a, b) => b.computedScore - a.computedScore)
    .slice(0, 50)

  // Additional heavy computations
  const analytics = {
    totalProcessed: processed.length,
    avgScore: processed.reduce((sum, item) => sum + item.computedScore, 0) / processed.length,
    maxCubed: Math.max(...numbers.map(n => n.cubed)),
    primeCount: numbers.filter(n => n.isPrime).length,
    categoryBreakdown: numbers.reduce((acc, item) => {
      acc[item.category] = (acc[item.category] || 0) + 1
      return acc
    }, {} as Record<number, number>),
    complexSum: processed.reduce((sum, item) => sum + item.complexCalc, 0),
  }

  const endTime = performance.now()
  const processingTime = endTime - startTime

  return (
    <div className="max-w-6xl mx-auto p-6 space-y-6">
      <div className="mb-8 p-4 bg-gradient-to-r from-red-500 to-orange-500 text-white rounded-lg">
        <div className="flex items-center gap-3 mb-2">
          <div className="w-8 h-8 bg-red-700 rounded-lg flex items-center justify-center">
            <span className="text-white text-sm font-bold">☢️</span>
          </div>
          <h2 className="text-2xl font-bold">
            NUCLEAR STRESS TEST
          </h2>
        </div>
        <p className="text-red-100 text-sm">
          Testing extreme computational performance with
          {' '}
          {itemCount.toLocaleString()}
          {' '}
          items
        </p>
        <div className="mt-4 text-sm bg-red-700 bg-opacity-50 px-3 py-2 rounded">
          <strong>🔥 Processing Time:</strong>
          {' '}
          {processingTime}
          ms
        </div>
        <div className="mt-2 text-xs text-red-200">
          Prime calculations • Factorial computations • Trigonometric operations • Deep object processing
        </div>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div className="bg-red-50 border border-red-200 p-4 rounded-lg">
          <h3 className="font-semibold text-red-900">Total Items</h3>
          <div className="text-2xl font-bold text-red-600">{itemCount.toLocaleString()}</div>
        </div>
        <div className="bg-orange-50 border border-orange-200 p-4 rounded-lg">
          <h3 className="font-semibold text-orange-900">Filtered</h3>
          <div className="text-2xl font-bold text-orange-600">{analytics.totalProcessed}</div>
        </div>
        <div className="bg-yellow-50 border border-yellow-200 p-4 rounded-lg">
          <h3 className="font-semibold text-yellow-900">Primes Found</h3>
          <div className="text-2xl font-bold text-yellow-600">{analytics.primeCount}</div>
        </div>
        <div className="bg-purple-50 border border-purple-200 p-4 rounded-lg">
          <h3 className="font-semibold text-purple-900">Ops/Second</h3>
          <div className="text-2xl font-bold text-purple-600">
            {Math.round(itemCount / (processingTime / 1000)).toLocaleString()}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div className="bg-white border rounded-lg p-6">
          <h2 className="text-xl font-bold mb-4">🧮 Computation Results</h2>
          <div className="space-y-3 text-sm">
            <div className="flex justify-between">
              <span className="font-medium">Average Score:</span>
              <span>{analytics.avgScore.toFixed(4)}</span>
            </div>
            <div className="flex justify-between">
              <span className="font-medium">Max Cubed Value:</span>
              <span>{analytics.maxCubed.toLocaleString()}</span>
            </div>
            <div className="flex justify-between">
              <span className="font-medium">Complex Sum:</span>
              <span>{analytics.complexSum.toLocaleString()}</span>
            </div>
            <div className="flex justify-between">
              <span className="font-medium">Processing Rate:</span>
              <span>
                {(itemCount / processingTime).toFixed(0)}
                {' '}
                items/ms
              </span>
            </div>
          </div>
        </div>

        <div className="bg-white border rounded-lg p-6">
          <h2 className="text-xl font-bold mb-4">📊 Category Distribution</h2>
          <div className="grid grid-cols-2 gap-3 text-sm">
            {Object.entries(analytics.categoryBreakdown).map(([category, count]) => (
              <div key={category} className="flex justify-between bg-gray-50 p-2 rounded">
                <span>
                  Cat
                  {category}
                  :
                </span>
                <span className="font-bold">{count.toLocaleString()}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="bg-white border rounded-lg p-6">
        <h2 className="text-xl font-bold mb-4">🏆 Top Computational Results</h2>
        <div className="overflow-x-auto">
          <table className="w-full text-xs">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-2 py-2 text-left">ID</th>
                <th className="px-2 py-2 text-left">Value</th>
                <th className="px-2 py-2 text-left">Squared</th>
                <th className="px-2 py-2 text-left">Cubed</th>
                <th className="px-2 py-2 text-left">Prime?</th>
                <th className="px-2 py-2 text-left">Score</th>
                <th className="px-2 py-2 text-left">Complex</th>
                <th className="px-2 py-2 text-left">Trig</th>
              </tr>
            </thead>
            <tbody>
              {processed.slice(0, 15).map(item => (
                <tr key={item.id} className="border-t">
                  <td className="px-2 py-2 font-mono">{item.id}</td>
                  <td className="px-2 py-2">{item.value}</td>
                  <td className="px-2 py-2">{item.squared.toLocaleString()}</td>
                  <td className="px-2 py-2">{item.cubed.toLocaleString()}</td>
                  <td className="px-2 py-2">
                    <span className={`px-1 py-0.5 rounded text-xs ${
                      item.isPrime ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-600'
                    }`}
                    >
                      {item.isPrime ? '✓' : '✗'}
                    </span>
                  </td>
                  <td className="px-2 py-2">{item.computedScore.toFixed(2)}</td>
                  <td className="px-2 py-2">{item.complexCalc.toLocaleString()}</td>
                  <td className="px-2 py-2">{item.trigResult.toFixed(1)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      <div className="bg-gradient-to-r from-gray-800 to-gray-900 text-white p-6 rounded-lg">
        <h2 className="text-xl font-bold mb-4">⚡ Performance Analysis</h2>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
          <div>
            <div className="text-gray-300">Total Operations:</div>
            <div className="text-2xl font-bold">{(itemCount * 8).toLocaleString()}</div>
            <div className="text-xs text-gray-400">8 ops per item</div>
          </div>
          <div>
            <div className="text-gray-300">Memory Processed:</div>
            <div className="text-2xl font-bold">
              {Math.round(itemCount * 200 / 1024 / 1024)}
              MB
            </div>
            <div className="text-xs text-gray-400">~200 bytes per item</div>
          </div>
          <div>
            <div className="text-gray-300">Efficiency:</div>
            <div className="text-2xl font-bold">{(itemCount / processingTime * 1000).toFixed(0)}</div>
            <div className="text-xs text-gray-400">items/second</div>
          </div>
        </div>
      </div>

      <div className="text-center text-sm text-gray-500 pt-6 border-t">
        🚀 Generated at
        {' '}
        {new Date().toLocaleString()}
        {' '}
        •
        Processed
        {' '}
        {itemCount.toLocaleString()}
        {' '}
        items with
        {' '}
        {(itemCount * 8).toLocaleString()}
        {' '}
        operations in
        {' '}
        {processingTime}
        ms
        <br />
        <span className="text-xs text-red-600 font-bold">
          NUCLEAR MODE: Prime detection • Factorial computation • Trigonometric calculations • Deep object processing
        </span>
      </div>
    </div>
  )
}
