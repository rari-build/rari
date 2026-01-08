'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { DEFAULT_DEVICE_SIZES, DEFAULT_IMAGE_SIZES } from './constants'

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
  if (format)
    params.set('f', format)
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

  const [blurComplete, setBlurComplete] = useState(false)
  const [showAltText, setShowAltText] = useState(false)
  const imgRef = useRef<HTMLImageElement>(null)
  const onLoadRef = useRef(onLoad)
  const pictureRef = useRef<HTMLPictureElement>(null)

  useEffect(() => {
    onLoadRef.current = onLoad
  }, [onLoad])

  const handleLoad = useCallback(
    (event: React.SyntheticEvent<HTMLImageElement>) => {
      const img = event.currentTarget

      if (img.src && img.complete) {
        if (placeholder === 'blur')
          setBlurComplete(true)

        if (onLoadRef.current)
          onLoadRef.current(event)
      }
    },
    [placeholder],
  )

  const handleError = useCallback(
    (event: React.SyntheticEvent<HTMLImageElement>) => {
      setShowAltText(true)
      if (placeholder === 'blur')
        setBlurComplete(true)

      if (onError)
        onError(event)
    },
    [placeholder, onError],
  )

  useEffect(() => {
    if (priority || unoptimized || loading === 'eager')
      return

    const img = imgRef.current
    if (!img)
      return

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            observer.unobserve(img)
          }
        })
      },
      {
        rootMargin: '50px',
      },
    )

    observer.observe(img)

    return () => {
      observer.disconnect()
    }
  }, [priority, unoptimized, loading])

  const imgStyle: React.CSSProperties = {
    ...style,
    ...(fill && {
      position: 'absolute',
      inset: 0,
      width: '100%',
      height: '100%',
      objectFit: 'cover',
    }),
    ...(placeholder === 'blur' && imgBlurDataURL && !blurComplete && {
      backgroundImage: `url(${imgBlurDataURL})`,
      backgroundSize: 'cover',
      backgroundPosition: 'center',
      filter: 'blur(20px)',
      transition: 'filter 0.3s ease-out',
    }),
    ...(placeholder === 'blur' && blurComplete && {
      filter: 'none',
      transition: 'filter 0.3s ease-out',
    }),
  }

  if (unoptimized || formats.length === 0) {
    return (
      <img
        ref={imgRef}
        src={imgSrc}
        alt={showAltText ? alt : ''}
        width={fill ? undefined : imgWidth}
        height={fill ? undefined : imgHeight}
        loading={priority ? 'eager' : loading}
        fetchPriority={priority ? 'high' : 'auto'}
        decoding={priority ? 'sync' : 'async'}
        onLoad={handleLoad}
        onError={handleError}
        style={imgStyle}
        className={className}
      />
    )
  }

  const sizesArray = fill ? DEFAULT_IMAGE_SIZES : DEFAULT_DEVICE_SIZES
  const defaultWidth = imgWidth || 1920

  const buildSrcSet = (format?: 'avif' | 'webp') =>
    sizesArray.map(w => `${buildImageUrl(imgSrc, w, quality, format)} ${w}w`).join(', ')

  const imgElement = (
    <img
      ref={imgRef}
      src={buildImageUrl(imgSrc, defaultWidth, quality)}
      srcSet={buildSrcSet()}
      sizes={sizes}
      alt={showAltText ? alt : ''}
      width={fill ? undefined : imgWidth}
      height={fill ? undefined : imgHeight}
      loading={priority ? 'eager' : loading}
      fetchPriority={priority ? 'high' : 'auto'}
      decoding={priority ? 'sync' : 'async'}
      onLoad={handleLoad}
      onError={handleError}
      style={imgStyle}
      className={className}
    />
  )

  if (formats.length === 1 && !formats.includes('avif') && !formats.includes('webp'))
    return imgElement

  return (
    <picture ref={pictureRef}>
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
