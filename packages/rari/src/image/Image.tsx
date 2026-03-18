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

function extractImageProps(src: string | StaticImageData, width?: number, height?: number, blurDataURL?: string) {
  const imgSrc = typeof src === 'string' ? src : src.src
  const imgWidth = width ?? (typeof src !== 'string' ? src.width : undefined)
  const imgHeight = height ?? (typeof src !== 'string' ? src.height : undefined)
  const imgBlurDataURL = blurDataURL ?? (typeof src !== 'string' ? src.blurDataURL : undefined)

  return { imgSrc, imgWidth, imgHeight, imgBlurDataURL }
}

function useImagePreload(
  shouldPreload: boolean,
  finalSrc: string,
  imgWidth: number | undefined,
  quality: number,
  sizes: string | undefined,
  loader: ImageProps['loader'],
  unoptimized: boolean,
) {
  useEffect(() => {
    if (!shouldPreload)
      return

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
      if (link.parentNode === document.head)
        document.head.removeChild(link)
    }
  }, [shouldPreload, finalSrc, imgWidth, quality, sizes, loader, unoptimized])
}

function useImageLazyLoad(
  imgRef: React.RefObject<HTMLImageElement | null>,
  shouldPreload: boolean,
  unoptimized: boolean,
  loading: 'lazy' | 'eager',
) {
  const shouldLoadImmediately = shouldPreload || unoptimized || loading === 'eager'
  const [hasIntersected, setHasIntersected] = useState(false)

  useEffect(() => {
    if (shouldLoadImmediately)
      return

    const img = imgRef.current
    if (!img)
      return

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            setHasIntersected(true)
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
  }, [imgRef, shouldLoadImmediately])

  return shouldLoadImmediately || hasIntersected
}

function buildImageStyle(
  style: React.CSSProperties | undefined,
  fill: boolean,
  placeholder: 'blur' | 'empty',
  imgBlurDataURL: string | undefined,
  blurComplete: boolean,
): React.CSSProperties {
  return {
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
}

function buildSrcSetString(
  sizesArray: number[],
  finalSrc: string,
  quality: number,
  format: ImageFormat | undefined,
  loader: ImageProps['loader'],
  useDprDescriptors: boolean = false,
): string {
  if (loader) {
    if (useDprDescriptors)
      return sizesArray.map((w, i) => `${loader({ src: finalSrc, width: w, quality })} ${i + 1}x`).join(', ')

    return sizesArray.map(w => `${loader({ src: finalSrc, width: w, quality })} ${w}w`).join(', ')
  }

  if (useDprDescriptors)
    return sizesArray.map((w, i) => `${buildImageUrl(finalSrc, w, quality, format)} ${i + 1}x`).join(', ')

  return sizesArray.map(w => `${buildImageUrl(finalSrc, w, quality, format)} ${w}w`).join(', ')
}

function UnoptimizedImage({
  imgRef,
  finalSrc,
  imgWidth,
  quality,
  loader,
  alt,
  fill,
  imgHeight,
  shouldPreload,
  loading,
  imgDecoding,
  handleLoad,
  handleError,
  imgStyle,
  className,
  isVisible,
}: {
  imgRef: React.RefObject<HTMLImageElement | null>
  finalSrc: string
  imgWidth: number | undefined
  quality: number
  loader: ImageProps['loader']
  alt: string
  fill: boolean
  imgHeight: number | undefined
  shouldPreload: boolean
  loading: 'lazy' | 'eager'
  imgDecoding: 'async' | 'sync' | 'auto'
  handleLoad: (event: React.SyntheticEvent<HTMLImageElement>) => void
  handleError: (event: React.SyntheticEvent<HTMLImageElement>) => void
  imgStyle: React.CSSProperties
  className: string | undefined
  isVisible: boolean
}) {
  const finalImgSrc = loader
    ? loader({ src: finalSrc, width: imgWidth || 1920, quality })
    : finalSrc

  return (
    <img
      ref={imgRef}
      src={isVisible ? finalImgSrc : undefined}
      alt={alt}
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

function OptimizedImage({
  imgRef,
  pictureRef,
  finalSrc,
  imgWidth,
  quality,
  loader,
  sizes,
  alt,
  fill,
  imgHeight,
  shouldPreload,
  loading,
  imgDecoding,
  handleLoad,
  handleError,
  imgStyle,
  className,
  isVisible,
}: {
  imgRef: React.RefObject<HTMLImageElement | null>
  pictureRef: React.RefObject<HTMLPictureElement | null>
  finalSrc: string
  imgWidth: number | undefined
  quality: number
  loader: ImageProps['loader']
  sizes: string | undefined
  alt: string
  fill: boolean
  imgHeight: number | undefined
  shouldPreload: boolean
  loading: 'lazy' | 'eager'
  imgDecoding: 'async' | 'sync' | 'auto'
  handleLoad: (event: React.SyntheticEvent<HTMLImageElement>) => void
  handleError: (event: React.SyntheticEvent<HTMLImageElement>) => void
  imgStyle: React.CSSProperties
  className: string | undefined
  isVisible: boolean
}) {
  const defaultWidth = imgWidth || 1920
  const hasFixedWidth = !!imgWidth
  const sizesArray = hasFixedWidth ? [imgWidth, imgWidth * 2, imgWidth * 3] : DEFAULT_DEVICE_SIZES

  const mainSrc = loader
    ? loader({ src: finalSrc, width: defaultWidth, quality })
    : buildImageUrl(finalSrc, defaultWidth, quality)

  const shouldUseSrcSet = true

  const imgElement = (
    <img
      ref={imgRef}
      src={isVisible ? mainSrc : undefined}
      srcSet={isVisible && shouldUseSrcSet ? buildSrcSetString(sizesArray, finalSrc, quality, undefined, loader, hasFixedWidth) : undefined}
      sizes={hasFixedWidth ? undefined : sizes}
      alt={alt}
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
      {isVisible && !loader && DEFAULT_FORMATS.includes('avif') && (
        <source
          type="image/avif"
          srcSet={buildSrcSetString(sizesArray, finalSrc, quality, 'avif', loader, hasFixedWidth)}
          sizes={hasFixedWidth ? undefined : sizes}
        />
      )}
      {isVisible && !loader && DEFAULT_FORMATS.includes('webp') && (
        <source
          type="image/webp"
          srcSet={buildSrcSetString(sizesArray, finalSrc, quality, 'webp', loader, hasFixedWidth)}
          sizes={hasFixedWidth ? undefined : sizes}
        />
      )}
      {imgElement}
    </picture>
  )
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
  const { imgSrc, imgWidth, imgHeight, imgBlurDataURL } = extractImageProps(src, width, height, blurDataURL)
  const finalSrc = overrideSrc || imgSrc
  const shouldPreload = preload
  const imgDecoding = decoding || (preload ? 'sync' : 'async')

  const [blurComplete, setBlurComplete] = useState(false)
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
      if (placeholder === 'blur')
        setBlurComplete(true)

      if (onError)
        onError(event)
    },
    [placeholder, onError],
  )

  useImagePreload(shouldPreload, finalSrc, imgWidth, quality, sizes, loader, unoptimized)
  const isVisible = useImageLazyLoad(imgRef, shouldPreload, unoptimized, loading)

  const imgStyle = buildImageStyle(style, fill, placeholder, imgBlurDataURL, blurComplete)

  const commonProps = {
    imgRef,
    finalSrc,
    imgWidth,
    quality,
    loader,
    alt,
    fill,
    imgHeight,
    shouldPreload,
    loading,
    imgDecoding,
    handleLoad,
    handleError,
    imgStyle,
    className,
    isVisible,
  }

  if (unoptimized) {
    return <UnoptimizedImage {...commonProps} />
  }

  return <OptimizedImage {...commonProps} pictureRef={pictureRef} sizes={sizes} />
}
