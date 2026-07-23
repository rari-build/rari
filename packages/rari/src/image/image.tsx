'use client'

import type { ImageFormat } from './constants'
import { useCallback, useEffect, useRef, useState } from 'react'
import { DEFAULT_DEVICE_SIZES, DEFAULT_FORMATS } from './constants'

export interface ImageProps {
  readonly src: string | StaticImageData
  readonly alt: string
  readonly width?: number
  readonly height?: number
  readonly quality?: number
  readonly preload?: boolean
  readonly loading?: 'lazy' | 'eager'
  readonly placeholder?: 'blur' | 'empty'
  readonly blurDataURL?: string
  readonly fill?: boolean
  readonly sizes?: string
  readonly style?: React.CSSProperties
  readonly className?: string
  readonly onLoad?: (event: React.SyntheticEvent<HTMLImageElement>) => void
  readonly onError?: (event: React.SyntheticEvent<HTMLImageElement>) => void
  readonly unoptimized?: boolean
  readonly loader?: (props: Readonly<{ src: string; width: number; quality: number }>) => string
  readonly overrideSrc?: string
  readonly decoding?: 'async' | 'sync' | 'auto'
}

export interface StaticImageData {
  readonly src: string
  readonly height: number
  readonly width: number
  readonly blurDataURL?: string
}

function buildImageUrl(src: string, width: number, quality: number, format?: ImageFormat): string {
  const params = new URLSearchParams()
  params.set('url', src)
  params.set('w', width.toString())
  params.set('q', quality.toString())
  if (format) params.set('f', format)

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
  const imgWidth =
    width != null && width !== 0 ? width : typeof src !== 'string' ? src.width : undefined
  const imgHeight =
    height != null && height !== 0 ? height : typeof src !== 'string' ? src.height : undefined
  const imgBlurDataURL =
    blurDataURL != null && blurDataURL !== ''
      ? blurDataURL
      : typeof src !== 'string'
        ? src.blurDataURL
        : undefined
  const finalSrc = overrideSrc != null && overrideSrc !== '' ? overrideSrc : imgSrc
  const shouldPreload = preload
  const imgDecoding = decoding ?? (preload ? 'sync' : 'async')

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
        if (placeholder === 'blur') setBlurComplete(true)

        if (onLoadRef.current) onLoadRef.current(event)
      }
    },
    [placeholder],
  )

  const handleError = useCallback(
    (event: React.SyntheticEvent<HTMLImageElement>) => {
      setShowAltText(true)
      if (placeholder === 'blur') setBlurComplete(true)

      if (onError) onError(event)
    },
    [placeholder, onError],
  )

  useEffect(() => {
    if (!shouldPreload) return undefined

    const link = document.createElement('link')
    link.rel = 'preload'
    link.as = 'image'
    if (loader)
      link.href = loader({
        src: finalSrc,
        width: imgWidth != null && imgWidth !== 0 ? imgWidth : 1920,
        quality,
      })
    else if (unoptimized) link.href = finalSrc
    else
      link.href = buildImageUrl(
        finalSrc,
        imgWidth != null && imgWidth !== 0 ? imgWidth : 1920,
        quality,
      )
    if (sizes != null && sizes !== '') link.setAttribute('imagesizes', sizes)
    document.head.appendChild(link)

    return () => {
      if (link.parentNode === document.head) document.head.removeChild(link)
    }
  }, [shouldPreload, finalSrc, imgWidth, quality, sizes, loader, unoptimized])

  useEffect(() => {
    if (shouldPreload || unoptimized || loading === 'eager') return undefined

    const img = imgRef.current
    if (!img) return undefined

    const observer = new IntersectionObserver(
      entries => {
        entries.forEach(entry => {
          if (entry.isIntersecting) observer.unobserve(img)
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
    ...(placeholder === 'blur' &&
      imgBlurDataURL != null &&
      imgBlurDataURL !== '' &&
      !blurComplete && {
        backgroundImage: `url(${imgBlurDataURL})`,
        backgroundSize: 'cover',
        backgroundPosition: 'center',
        filter: 'blur(20px)',
        transition: 'filter 0.3s ease-out',
      }),
    ...(placeholder === 'blur' &&
      blurComplete && {
        filter: 'none',
        transition: 'filter 0.3s ease-out',
      }),
  }

  if (unoptimized) {
    const finalImgSrc = loader
      ? loader({
          src: finalSrc,
          width: imgWidth != null && imgWidth !== 0 ? imgWidth : 1920,
          quality,
        })
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

  const defaultWidth = imgWidth != null && imgWidth !== 0 ? imgWidth : 1920
  const sizesArray = imgWidth != null && imgWidth !== 0 ? [imgWidth] : DEFAULT_DEVICE_SIZES

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

  if (!shouldUseSrcSet) return imgElement

  return (
    <picture ref={pictureRef}>
      {DEFAULT_FORMATS.includes('avif') && (
        <source type="image/avif" srcSet={buildSrcSet('avif')} sizes={sizes} />
      )}
      {DEFAULT_FORMATS.includes('webp') && (
        <source type="image/webp" srcSet={buildSrcSet('webp')} sizes={sizes} />
      )}
      {imgElement}
    </picture>
  )
}
