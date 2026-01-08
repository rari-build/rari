import { Image } from 'rari/image'

export default function TestImagePage() {
  return (
    <div style={{ padding: '2rem', maxWidth: '1200px', margin: '0 auto' }}>
      <h1 style={{ marginBottom: '2rem' }}>Rari Image Component Test</h1>

      <section style={{ marginBottom: '3rem' }}>
        <h2>1. Remote Image (Unsplash)</h2>
        <Image
          src="https://images.unsplash.com/photo-1682687220742-aba13b6e50ba"
          alt="Mountain landscape"
          width={800}
          height={600}
          quality={75}
        />
      </section>

      <section style={{ marginBottom: '3rem' }}>
        <h2>2. Priority Image (Above the fold)</h2>
        <Image
          src="https://images.unsplash.com/photo-1506905925346-21bda4d32df4"
          alt="Mountain view"
          width={1200}
          height={600}
          priority
          quality={75}
        />
      </section>

      <section style={{ marginBottom: '3rem' }}>
        <h2>3. Responsive Images</h2>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: '1rem' }}>
          <Image
            src="https://images.unsplash.com/photo-1506905925346-21bda4d32df4"
            alt="Image 1"
            width={400}
            height={300}
            sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
          />
          <Image
            src="https://images.unsplash.com/photo-1682687220742-aba13b6e50ba"
            alt="Image 2"
            width={400}
            height={300}
            sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
          />
          <Image
            src="https://images.unsplash.com/photo-1506905925346-21bda4d32df4"
            alt="Image 3"
            width={400}
            height={300}
            sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
          />
        </div>
      </section>

      <section style={{ marginBottom: '3rem' }}>
        <h2>4. Fill Mode</h2>
        <div style={{ position: 'relative', width: '100%', height: '400px', backgroundColor: '#f0f0f0' }}>
          <Image
            src="https://images.unsplash.com/photo-1682687220742-aba13b6e50ba"
            alt="Fill container"
            fill
            style={{ objectFit: 'cover' }}
          />
        </div>
      </section>

      <section style={{ marginBottom: '3rem' }}>
        <h2>5. Different Quality Levels</h2>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '1rem' }}>
          <div>
            <p>Quality: 25</p>
            <Image
              src="https://images.unsplash.com/photo-1506905925346-21bda4d32df4"
              alt="Quality 25"
              width={300}
              height={200}
              quality={25}
            />
          </div>
          <div>
            <p>Quality: 75 (default)</p>
            <Image
              src="https://images.unsplash.com/photo-1506905925346-21bda4d32df4"
              alt="Quality 75"
              width={300}
              height={200}
              quality={75}
            />
          </div>
          <div>
            <p>Quality: 100</p>
            <Image
              src="https://images.unsplash.com/photo-1506905925346-21bda4d32df4"
              alt="Quality 100"
              width={300}
              height={200}
              quality={100}
            />
          </div>
        </div>
      </section>

      <section style={{ marginBottom: '3rem' }}>
        <h2>6. Modern Formats (AVIF + WebP)</h2>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '1rem' }}>
          <div>
            <p>AVIF + WebP (default)</p>
            <Image
              src="https://images.unsplash.com/photo-1506905925346-21bda4d32df4"
              alt="AVIF and WebP"
              width={400}
              height={300}
              formats={['avif', 'webp']}
            />
          </div>
          <div>
            <p>AVIF only</p>
            <Image
              src="https://images.unsplash.com/photo-1682687220742-aba13b6e50ba"
              alt="AVIF only"
              width={400}
              height={300}
              formats={['avif']}
            />
          </div>
        </div>
      </section>

      <section style={{ marginBottom: '3rem' }}>
        <h2>7. Blur Placeholder</h2>
        <Image
          src="https://images.unsplash.com/photo-1506905925346-21bda4d32df4"
          alt="Blur placeholder example"
          width={800}
          height={600}
          placeholder="blur"
          blurDataURL="data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAYEBQYFBAYGBQYHBwYIChAKCgkJChQODwwQFxQYGBcUFhYaHSUfGhsjHBYWICwgIyYnKSopGR8tMC0oMCUoKSj/2wBDAQcHBwoIChMKChMoGhYaKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCj/wAARCAAIAAoDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAb/xAAhEAACAQMDBQAAAAAAAAAAAAABAgMABAUGIWEREiMxUf/EABUBAQEAAAAAAAAAAAAAAAAAAAMF/8QAGhEAAgIDAAAAAAAAAAAAAAAAAAECEgMRkf/aAAwDAQACEQMRAD8AltJagyeH0AthI5xdrLcNM91BF5pX2HaH9bcfaSXWGaRmknyJckliyjqTzSlT54b6bk+h0R//2Q=="
        />
      </section>

      <section style={{ marginBottom: '3rem' }}>
        <h2>8. Unoptimized (Bypass)</h2>
        <Image
          src="https://images.unsplash.com/photo-1682687220742-aba13b6e50ba"
          alt="Unoptimized"
          width={600}
          height={400}
          unoptimized
        />
      </section>
    </div>
  )
}
