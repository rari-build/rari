'use client'

import { Image } from 'rari/image'
import { startTransition, Suspense, useState, ViewTransition } from 'react'

const IMAGE_COUNT = 2

function ImagePanel({ index }: { readonly index: number }) {
  if (index === 0) {
    return (
      <div className="rounded-lg overflow-hidden border border-gray-200 bg-white">
        <Image
          src="https://images.unsplash.com/photo-1576191919769-40424bb34367"
          alt="Joshua Tree landscape"
          width={1200}
          height={600}
          className="w-full h-auto"
        />
        <p className="p-3 text-sm text-gray-600">Joshua Tree landscape</p>
      </div>
    )
  }

  return (
    <div className="rounded-lg overflow-hidden border border-gray-200 bg-white">
      <Image
        src="https://images.unsplash.com/photo-1506905925346-21bda4d32df4"
        alt="Mountain landscape"
        width={1200}
        height={600}
        className="w-full h-auto"
      />
      <p className="p-3 text-sm text-gray-600">Mountain landscape</p>
    </div>
  )
}

export function ViewTransitionImageDemo() {
  const [index, setIndex] = useState(0)

  return (
    <div className="space-y-4">
      <button
        type="button"
        className="px-4 py-2 text-sm font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800"
        onClick={() => {
          startTransition(() => {
            setIndex(current => (current + 1) % IMAGE_COUNT)
          })
        }}
      >
        Swap image
      </button>

      <ViewTransition update="auto" default="none" key={index}>
        <Suspense fallback={<div className="h-48 rounded-lg bg-gray-100 animate-pulse" />}>
          <ImagePanel index={index} />
        </Suspense>
      </ViewTransition>
    </div>
  )
}
