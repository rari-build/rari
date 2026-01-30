'use client'

import type { ImageFormat } from './constants'
import { useCallback, useEffect, useRef, useState } from 'react'
import { DEFAULT_DEVICE_SIZES, DEFAULT_FORMATS } from './constants'

export interface ImageProps {
  src: string | StaticImageData
  alt: string
  width?: number
  height?: number
  quality?: number
  preload?: boolean
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
  loader?: (props: { src: string, width: number, quality: number }) => string
  overrideSrc?: string
  decoding?: 'async' | 'sync' | 'auto'
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
  format?: ImageFormat,
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
  preload = false,
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
  loader,
  overrideSrc,
  decoding,
}: ImageProps) {
  const imgSrc = typeof src === 'string' ? src : src.src
  const imgWidth = width || (typeof src !== 'string' ? src.width : undefined)
  const imgHeight = height || (typeof src !== 'string' ? src.height : undefined)
  const imgBlurDataURL = blurDataURL || (typeof src !== 'string' ? src.blurDataURL : undefined)
  const finalSrc = overrideSrc || imgSrc
  const shouldPreload = preload
  const imgDecoding = decoding || (preload ? 'sync' : 'async')

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
    if (shouldPreload) {
      const link = document.createElement('link')
      link.rel = 'preload'
      link.as = 'image'
      if (loader)
        link.href = loader({ src: finalSrc, width: imgWidth || 1920, quality })
      else if (unoptimized)
        link.href = finalSrc
      else
        link.href = buildImageUrl(finalSrc, imgWidth || 1920, quality)
      if (sizes)
        link.setAttribute('imagesizes', sizes)
      document.head.appendChild(link)

      return () => {
        document.head.removeChild(link)
      }
    }
  }, [shouldPreload, finalSrc, imgWidth, quality, sizes, loader, unoptimized])

  useEffect(() => {
    if (shouldPreload || unoptimized || loading === 'eager')
      return

    const img = imgRef.current
    if (!img)
      return

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting)
            observer.unobserve(img)
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
  }, [shouldPreload, unoptimized, loading])

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

  if (unoptimized) {
    const finalImgSrc = loader
      ? loader({ src: finalSrc, width: imgWidth || 1920, quality })
      : finalSrc

    return (
      <img
        ref={imgRef}
        src={finalImgSrc}
        alt={showAltText ? alt : ''}
        width={fill ? undefined : imgWidth}
        height={fill ? undefined : imgHeight}
        loading={shouldPreload ? 'eager' : loading}
        fetchPriority={shouldPreload ? 'high' : 'auto'}
        decoding={imgDecoding}
        onLoad={handleLoad}
        onError={handleError}
        style={imgStyle}
        className={className}
      />
    )
  }

  const defaultWidth = imgWidth || 1920
  const sizesArray = imgWidth ? [imgWidth] : DEFAULT_DEVICE_SIZES

  const buildSrcSet = (format?: ImageFormat) => {
    if (loader)
      return sizesArray.map(w => `${loader({ src: finalSrc, width: w, quality })} ${w}w`).join(', ')

    return sizesArray.map(w => `${buildImageUrl(finalSrc, w, quality, format)} ${w}w`).join(', ')
  }

  const mainSrc = loader
    ? loader({ src: finalSrc, width: defaultWidth, quality })
    : buildImageUrl(finalSrc, defaultWidth, quality)

  const shouldUseSrcSet = sizesArray.length > 1 || sizesArray[0] !== defaultWidth

  const imgElement = (
    <img
      ref={imgRef}
      src={mainSrc}
      srcSet={shouldUseSrcSet ? buildSrcSet() : undefined}
      sizes={shouldUseSrcSet ? sizes : undefined}
      alt={showAltText ? alt : ''}
      width={fill ? undefined : imgWidth}
      height={fill ? undefined : imgHeight}
      loading={shouldPreload ? 'eager' : loading}
      fetchPriority={shouldPreload ? 'high' : 'auto'}
      decoding={imgDecoding}
      onLoad={handleLoad}
      onError={handleError}
      style={imgStyle}
      className={className}
    />
  )

  if (!shouldUseSrcSet)
    return imgElement

  return (
    <picture ref={pictureRef}>
      {DEFAULT_FORMATS.includes('avif') && (
        <source
          type="image/avif"
          srcSet={buildSrcSet('avif')}
          sizes={sizes}
        />
      )}
      {DEFAULT_FORMATS.includes('webp') && (
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
