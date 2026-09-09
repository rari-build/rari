'use client'

import type { ImageFormat } from './constants'
import { useCallback, useEffect, useRef, useState } from 'react'
import { DEFAULT_DEVICE_SIZES, DEFAULT_FORMATS } from './constants'
import { resolveOptimizedSizePlan } from './size-plan'

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
  const intrinsicWidth = typeof src !== 'string' ? src.width : undefined
  const intrinsicHeight = typeof src !== 'string' ? src.height : undefined
  const imgWidth =
    width != null && width !== 0
      ? width
      : !fill && intrinsicWidth != null && intrinsicWidth !== 0
        ? intrinsicWidth
        : undefined
  const imgHeight =
    height != null && height !== 0
      ? height
      : !fill && intrinsicHeight != null && intrinsicHeight !== 0
        ? intrinsicHeight
        : undefined
  const imgBlurDataURL =
    blurDataURL != null && blurDataURL !== ''
      ? blurDataURL
      : typeof src !== 'string'
        ? src.blurDataURL
        : undefined
  const finalSrc = overrideSrc != null && overrideSrc !== '' ? overrideSrc : imgSrc
  const shouldPreload = preload
  const imgDecoding = decoding ?? (preload ? 'sync' : 'async')
  const sizePlan = resolveOptimizedSizePlan({
    fill,
    width,
    intrinsicWidth,
  })
  const shouldUseSrcSet = sizePlan.widths.length > 1 || sizePlan.widths[0] !== sizePlan.defaultWidth

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

    const useResponsivePreload = shouldUseSrcSet && !unoptimized
    const preloadSizes =
      sizes != null && sizes !== '' ? sizes : useResponsivePreload || fill ? '100vw' : undefined
    const preloadAvifOnly =
      loader == null && DEFAULT_FORMATS.length === 1 && DEFAULT_FORMATS[0] === 'avif'
    const preloadFormat: ImageFormat | undefined = preloadAvifOnly ? 'avif' : undefined

    if (unoptimized) {
      link.href =
        loader != null ? loader({ src: finalSrc, width: sizePlan.defaultWidth, quality }) : finalSrc
    } else if (useResponsivePreload) {
      const srcSet = DEFAULT_DEVICE_SIZES.map(w =>
        loader != null
          ? `${loader({ src: finalSrc, width: w, quality })} ${w}w`
          : `${buildImageUrl(finalSrc, w, quality, preloadFormat)} ${w}w`,
      ).join(', ')
      link.href =
        loader != null
          ? loader({ src: finalSrc, width: sizePlan.defaultWidth, quality })
          : buildImageUrl(finalSrc, sizePlan.defaultWidth, quality, preloadFormat)
      link.setAttribute('imagesrcset', srcSet)
      if (preloadSizes != null) link.setAttribute('imagesizes', preloadSizes)
      if (preloadAvifOnly) link.type = 'image/avif'
    } else if (loader != null) {
      link.href = loader({
        src: finalSrc,
        width: sizePlan.defaultWidth,
        quality,
      })
      if (preloadSizes != null) link.setAttribute('imagesizes', preloadSizes)
    } else {
      link.href = buildImageUrl(finalSrc, sizePlan.defaultWidth, quality)
      if (preloadSizes != null) link.setAttribute('imagesizes', preloadSizes)
    }

    document.head.appendChild(link)

    return () => {
      if (link.parentNode === document.head) document.head.removeChild(link)
    }
  }, [
    shouldPreload,
    finalSrc,
    sizePlan.defaultWidth,
    quality,
    sizes,
    loader,
    unoptimized,
    fill,
    shouldUseSrcSet,
  ])

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
      objectFit: style?.objectFit ?? 'cover',
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
          width: sizePlan.defaultWidth,
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
        onLoad={placeholder === 'blur' || onLoad != null ? handleLoad : undefined}
        onError={handleError}
        style={imgStyle}
        className={className}
      />
    )
  }

  const sizesArray = sizePlan.widths
  const defaultWidth = sizePlan.defaultWidth

  const buildSrcSet = (format?: ImageFormat) => {
    if (loader)
      return sizesArray.map(w => `${loader({ src: finalSrc, width: w, quality })} ${w}w`).join(', ')

    return sizesArray.map(w => `${buildImageUrl(finalSrc, w, quality, format)} ${w}w`).join(', ')
  }

  const mainSrc = loader
    ? loader({ src: finalSrc, width: defaultWidth, quality })
    : buildImageUrl(finalSrc, defaultWidth, quality)

  const resolvedSizes =
    sizes != null && sizes !== '' ? sizes : shouldUseSrcSet ? '100vw' : undefined

  const imgElement = (
    <img
      ref={imgRef}
      src={mainSrc}
      srcSet={shouldUseSrcSet ? buildSrcSet() : undefined}
      sizes={shouldUseSrcSet ? resolvedSizes : undefined}
      alt={showAltText ? alt : ''}
      width={fill ? undefined : imgWidth}
      height={fill ? undefined : imgHeight}
      loading={shouldPreload ? 'eager' : loading}
      fetchPriority={shouldPreload ? 'high' : 'auto'}
      decoding={imgDecoding}
      onLoad={placeholder === 'blur' || onLoad != null ? handleLoad : undefined}
      onError={handleError}
      style={imgStyle}
      className={className}
    />
  )

  if (!shouldUseSrcSet) return imgElement

  return (
    <picture ref={pictureRef}>
      {DEFAULT_FORMATS.includes('avif') && (
        <source type="image/avif" srcSet={buildSrcSet('avif')} sizes={resolvedSizes} />
      )}
      {DEFAULT_FORMATS.includes('webp') && (
        <source type="image/webp" srcSet={buildSrcSet('webp')} sizes={resolvedSizes} />
      )}
      {imgElement}
    </picture>
  )
}
