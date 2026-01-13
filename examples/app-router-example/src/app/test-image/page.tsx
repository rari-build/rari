import { Image } from 'rari/image'

export default function TestImagePage() {
  return (
    <div className="space-y-8">
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 md:p-12">
        <div className="flex items-center gap-3 mb-4">
          <h1 className="text-4xl md:text-5xl font-bold text-gray-900">
            Image Optimization
          </h1>
          <span className="text-3xl">🖼️</span>
        </div>

        <p className="text-lg text-gray-600 mb-6 max-w-3xl leading-relaxed">
          High-performance image optimization powered by Rust. Automatic format
          conversion, responsive sizing, and lazy loading out of the box.
        </p>

        <div className="flex flex-wrap gap-3">
          <span className="inline-flex items-center px-3 py-1 text-xs font-medium text-green-700 bg-green-50 border border-green-200 rounded-full">
            ✓ WebP/AVIF Support
          </span>
          <span className="inline-flex items-center px-3 py-1 text-xs font-medium text-blue-700 bg-blue-50 border border-blue-200 rounded-full">
            ✓ Lazy Loading
          </span>
          <span className="inline-flex items-center px-3 py-1 text-xs font-medium text-purple-700 bg-purple-50 border border-purple-200 rounded-full">
            ✓ Rust Powered
          </span>
        </div>
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <div className="flex items-center gap-2 mb-4">
          <span className="text-2xl">🌐</span>
          <h2 className="text-2xl font-bold text-gray-900">Remote Images</h2>
        </div>
        <p className="text-sm text-gray-600 mb-6">
          Automatically optimized images from external sources with caching and format conversion
        </p>
        <div className="rounded-lg overflow-hidden border border-gray-200">
          <Image
            src="https://images.unsplash.com/photo-1576191919769-40424bb34367"
            alt="Joshua Tree landscape"
            width={1200}
            height={600}
            quality={75}
            className="w-full h-auto"
          />
        </div>
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <div className="flex items-center gap-2 mb-4">
          <span className="text-2xl">⚡</span>
          <h2 className="text-2xl font-bold text-gray-900">Priority Loading</h2>
        </div>
        <p className="text-sm text-gray-600 mb-6">
          Above-the-fold images with preload for instant visibility
        </p>
        <div className="rounded-lg overflow-hidden border border-gray-200">
          <Image
            src="https://images.unsplash.com/photo-1654652602865-53efa00c3e05"
            alt="A view of a beach with a mountain in the background"
            width={1200}
            height={600}
            preload
            quality={75}
            className="w-full h-auto"
          />
        </div>
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <div className="flex items-center gap-2 mb-4">
          <span className="text-2xl">📁</span>
          <h2 className="text-2xl font-bold text-gray-900">Local Images</h2>
        </div>
        <p className="text-sm text-gray-600 mb-6">
          Optimized images from your public folder with automatic sizing
        </p>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div className="space-y-3">
            <div className="text-sm font-medium text-gray-700">Standard Loading</div>
            <div className="rounded-lg overflow-hidden border border-gray-200">
              <Image
                src="/images/silhouette-of-trees.jpg"
                alt="Silhouette of palm trees at night"
                width={600}
                height={400}
                quality={75}
                className="w-full h-auto"
              />
            </div>
          </div>
          <div className="space-y-3">
            <div className="text-sm font-medium text-gray-700">Priority Loading</div>
            <div className="rounded-lg overflow-hidden border border-gray-200">
              <Image
                src="/images/silhouette-of-trees.jpg"
                alt="Silhouette of palm trees at night"
                width={600}
                height={400}
                preload
                quality={75}
                className="w-full h-auto"
              />
            </div>
          </div>
        </div>
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <div className="flex items-center gap-2 mb-4">
          <span className="text-2xl">📱</span>
          <h2 className="text-2xl font-bold text-gray-900">Responsive Grid</h2>
        </div>
        <p className="text-sm text-gray-600 mb-6">
          Adaptive sizing based on viewport with automatic srcset generation
        </p>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div className="rounded-lg overflow-hidden border border-gray-200 hover:shadow-md transition-shadow">
            <Image
              src="https://images.unsplash.com/photo-1654652602865-53efa00c3e05"
              alt="A view of a beach with a mountain in the background"
              width={400}
              height={300}
              sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
              className="w-full h-auto"
            />
          </div>
          <div className="rounded-lg overflow-hidden border border-gray-200 hover:shadow-md transition-shadow">
            <Image
              src="/images/silhouette-of-trees.jpg"
              alt="Silhouette of palm trees at night"
              width={400}
              height={300}
              sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
              className="w-full h-auto"
            />
          </div>
          <div className="rounded-lg overflow-hidden border border-gray-200 hover:shadow-md transition-shadow">
            <Image
              src="https://images.unsplash.com/photo-1576191919769-40424bb34367"
              alt="Joshua Tree landscape"
              width={400}
              height={300}
              sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
              className="w-full h-auto"
            />
          </div>
        </div>
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <div className="flex items-center gap-2 mb-4">
          <span className="text-2xl">🎯</span>
          <h2 className="text-2xl font-bold text-gray-900">Fill Mode</h2>
        </div>
        <p className="text-sm text-gray-600 mb-6">
          Container-based sizing with object-fit for flexible layouts
        </p>
        <div className="relative w-full h-96 rounded-lg overflow-hidden border border-gray-200">
          <Image
            src="/images/silhouette-of-trees.jpg"
            alt="Silhouette of palm trees at night"
            fill
            style={{ objectFit: 'cover' }}
          />
        </div>
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <div className="flex items-center gap-2 mb-4">
          <span className="text-2xl">🎨</span>
          <h2 className="text-2xl font-bold text-gray-900">Quality Comparison</h2>
        </div>
        <p className="text-sm text-gray-600 mb-6">
          Balance file size and visual quality with adjustable compression
        </p>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-gray-700">Low Quality</span>
              <span className="text-xs font-semibold text-gray-500 bg-gray-100 px-2 py-1 rounded">25%</span>
            </div>
            <div className="rounded-lg overflow-hidden border border-gray-200">
              <Image
                src="/images/silhouette-of-trees.jpg"
                alt="Silhouette of palm trees at night"
                width={400}
                height={267}
                quality={25}
                className="w-full h-auto"
              />
            </div>
            <p className="text-xs text-gray-500">Smaller file size, visible compression</p>
          </div>
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-gray-700">Standard Quality</span>
              <span className="text-xs font-semibold text-indigo-600 bg-indigo-50 px-2 py-1 rounded">75%</span>
            </div>
            <div className="rounded-lg overflow-hidden border-2 border-indigo-200">
              <Image
                src="/images/silhouette-of-trees.jpg"
                alt="Silhouette of palm trees at night"
                width={400}
                height={267}
                quality={75}
                className="w-full h-auto"
              />
            </div>
            <p className="text-xs text-gray-500">Recommended balance (default)</p>
          </div>
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-gray-700">High Quality</span>
              <span className="text-xs font-semibold text-gray-500 bg-gray-100 px-2 py-1 rounded">100%</span>
            </div>
            <div className="rounded-lg overflow-hidden border border-gray-200">
              <Image
                src="/images/silhouette-of-trees.jpg"
                alt="Silhouette of palm trees at night"
                width={400}
                height={267}
                quality={100}
                className="w-full h-auto"
              />
            </div>
            <p className="text-xs text-gray-500">Larger file size, maximum quality</p>
          </div>
        </div>
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <div className="flex items-center gap-2 mb-4">
          <span className="text-2xl">✨</span>
          <h2 className="text-2xl font-bold text-gray-900">Blur Placeholder</h2>
        </div>
        <p className="text-sm text-gray-600 mb-6">
          Smooth loading experience with low-quality image placeholders
        </p>
        <div className="rounded-lg overflow-hidden border border-gray-200">
          <Image
            src="/images/silhouette-of-trees.jpg"
            alt="Silhouette of palm trees at night"
            width={1200}
            height={800}
            placeholder="blur"
            blurDataURL="data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAYEBQYFBAYGBQYHBwYIChAKCgkJChQODwwQFxQYGBcUFhYaHSUfGhsjHBYWICwgIyYnKSopGR8tMC0oMCUoKSj/2wBDAQcHBwoIChMKChMoGhYaKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCj/wAARCAAIAAoDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAb/xAAhEAACAQMDBQAAAAAAAAAAAAABAgMABAUGIWEREiMxUf/EABUBAQEAAAAAAAAAAAAAAAAAAAMF/8QAGhEAAgIDAAAAAAAAAAAAAAAAAAECEgMRkf/aAAwDAQACEQMRAD8AltJagyeH0AthI5xdrLcNM91BF5pX2HaH9bcfaSXWGaRmknyJckliyjqTzSlT54b6bk+h0R//2Q=="
            className="w-full h-auto"
          />
        </div>
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <div className="flex items-center gap-2 mb-4">
          <span className="text-2xl">🔧</span>
          <h2 className="text-2xl font-bold text-gray-900">Unoptimized Mode</h2>
        </div>
        <p className="text-sm text-gray-600 mb-6">
          Bypass optimization when needed for special use cases
        </p>
        <div className="rounded-lg overflow-hidden border border-gray-200 max-w-2xl">
          <Image
            src="/images/silhouette-of-trees.jpg"
            alt="Silhouette of palm trees at night"
            width={600}
            height={400}
            unoptimized
            className="w-full h-auto"
          />
        </div>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Image Optimization | Rari App Router',
  description: 'High-performance image optimization powered by Rust',
}
