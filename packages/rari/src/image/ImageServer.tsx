export interface ImageProps {
  src: string | StaticImageData
  alt: string
  width?: number
  height?: number
  quality?: number
  priority?: boolean
  loading?: 'lazy' | 'eager'
  placeholder?: 'blur' | 'empty'
  blurDataURL?: string
  fill?: boolean
  sizes?: string
  style?: React.CSSProperties
  className?: string
  onLoad?: (event: React.SyntheticEvent<HTMLImageElement>) => void
  onError?: (event: React.SyntheticEvent<HTMLImageElement>) => void
  unoptimized?: boolean
  formats?: ('avif' | 'webp')[]
}

export interface StaticImageData {
  src: string
  height: number
  width: number
  blurDataURL?: string
}

const DEVICE_SIZES = [640, 750, 828, 1080, 1200, 1920, 2048, 3840]
const IMAGE_SIZES = [16, 32, 48, 64, 96, 128, 256, 384]

function buildImageUrl(
  src: string,
  width: number,
  quality: number,
  format?: 'avif' | 'webp',
): string {
  const params = new URLSearchParams()
  params.set('url', src)
  params.set('w', width.toString())
  params.set('q', quality.toString())
  if (format) {
    params.set('f', format)
  }
  return `/_rari/image?${params}`
}

export function Image({
  src,
  alt,
  width,
  height,
  quality = 75,
  priority = false,
  loading = 'lazy',
  placeholder = 'empty',
  blurDataURL,
  fill = false,
  sizes,
  style,
  className,
  onLoad,
  onError,
  unoptimized = false,
  formats = ['avif', 'webp'],
}: ImageProps) {
  const imgSrc = typeof src === 'string' ? src : src.src
  const imgWidth = width || (typeof src !== 'string' ? src.width : undefined)
  const imgHeight = height || (typeof src !== 'string' ? src.height : undefined)
  const imgBlurDataURL = blurDataURL || (typeof src !== 'string' ? src.blurDataURL : undefined)

  const imgStyle: React.CSSProperties = {
    ...style,
    ...(fill && {
      position: 'absolute',
      inset: 0,
      width: '100%',
      height: '100%',
      objectFit: 'cover',
    }),
  }

  if (placeholder === 'blur' && imgBlurDataURL) {
    imgStyle.backgroundImage = `url(${imgBlurDataURL})`
    imgStyle.backgroundSize = 'cover'
  }

  if (unoptimized || formats.length === 0) {
    return (
      <img
        src={imgSrc}
        alt={alt}
        width={fill ? undefined : imgWidth}
        height={fill ? undefined : imgHeight}
        loading={priority ? 'eager' : loading}
        fetchPriority={priority ? 'high' : 'auto'}
        decoding={priority ? 'sync' : 'async'}
        onLoad={onLoad}
        onError={onError}
        style={imgStyle}
        className={className}
      />
    )
  }

  const sizesArray = fill ? IMAGE_SIZES : DEVICE_SIZES
  const defaultWidth = imgWidth || 1920

  const buildSrcSet = (format?: 'avif' | 'webp') =>
    sizesArray.map(w => `${buildImageUrl(imgSrc, w, quality, format)} ${w}w`).join(', ')

  const imgElement = (
    <img
      src={buildImageUrl(imgSrc, defaultWidth, quality)}
      srcSet={buildSrcSet()}
      sizes={sizes}
      alt={alt}
      width={fill ? undefined : imgWidth}
      height={fill ? undefined : imgHeight}
      loading={priority ? 'eager' : loading}
      fetchPriority={priority ? 'high' : 'auto'}
      decoding={priority ? 'sync' : 'async'}
      onLoad={onLoad}
      onError={onError}
      style={imgStyle}
      className={className}
    />
  )

  if (formats.length === 1 && !formats.includes('avif') && !formats.includes('webp')) {
    return imgElement
  }

  return (
    <picture>
      {formats.includes('avif') && (
        <source
          type="image/avif"
          srcSet={buildSrcSet('avif')}
          sizes={sizes}
        />
      )}
      {formats.includes('webp') && (
        <source
          type="image/webp"
          srcSet={buildSrcSet('webp')}
          sizes={sizes}
        />
      )}
      {imgElement}
    </picture>
  )
}
