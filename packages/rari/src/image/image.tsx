'use client'

import type { ImageFormat } from './constants'
import { useCallback, useEffect, useRef, useState } from 'react'
import {
  BLUR_PLACEHOLDER_QUALITY,
  BLUR_PLACEHOLDER_WIDTH,
  DEFAULT_DEVICE_SIZES,
  DEFAULT_FORMATS,
} from './constants'
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

function resolveLoaderOrBuiltUrl(
  loader: ImageProps['loader'],
  finalSrc: string,
  width: number,
  quality: number,
  format?: ImageFormat,
): string {
  return loader != null
    ? loader({ src: finalSrc, width, quality })
    : buildImageUrl(finalSrc, width, quality, format)
}

function resolvePreloadSizes(
  sizes: string | undefined,
  useResponsivePreload: boolean,
  fill: boolean,
): string | undefined {
  if (sizes != null && sizes !== '') return sizes
  return useResponsivePreload || fill ? '100vw' : undefined
}

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
function applyResponsivePreload(options: {
  readonly link: HTMLLinkElement
  readonly finalSrc: string
  readonly defaultWidth: number
  readonly quality: number
  readonly preloadSizes: string | undefined
  readonly loader: ImageProps['loader']
  readonly preloadFormat: ImageFormat | undefined
  readonly preloadAvifOnly: boolean
}): void {
  const {
    link,
    finalSrc,
    defaultWidth,
    quality,
    preloadSizes,
    loader,
    preloadFormat,
    preloadAvifOnly,
  } = options
  const srcSet = DEFAULT_DEVICE_SIZES.map(
    w => `${resolveLoaderOrBuiltUrl(loader, finalSrc, w, quality, preloadFormat)} ${w}w`,
  ).join(', ')
  link.href = resolveLoaderOrBuiltUrl(loader, finalSrc, defaultWidth, quality, preloadFormat)
  link.setAttribute('imagesrcset', srcSet)
  if (preloadSizes != null) link.setAttribute('imagesizes', preloadSizes)
  if (preloadAvifOnly) link.type = 'image/avif'
}

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
function configureImagePreloadLink(options: {
  readonly link: HTMLLinkElement
  readonly finalSrc: string
  readonly defaultWidth: number
  readonly quality: number
  readonly sizes: string | undefined
  readonly loader: ImageProps['loader']
  readonly unoptimized: boolean
  readonly fill: boolean
  readonly shouldUseSrcSet: boolean
}): void {
  const {
    link,
    finalSrc,
    defaultWidth,
    quality,
    sizes,
    loader,
    unoptimized,
    fill,
    shouldUseSrcSet,
  } = options
  const useResponsivePreload = shouldUseSrcSet && !unoptimized
  const preloadSizes = resolvePreloadSizes(sizes, useResponsivePreload, fill)
  const preloadAvifOnly =
    loader == null && DEFAULT_FORMATS.length === 1 && DEFAULT_FORMATS[0] === 'avif'
  const preloadFormat: ImageFormat | undefined = preloadAvifOnly ? 'avif' : undefined

  if (unoptimized) {
    link.href = loader != null ? loader({ src: finalSrc, width: defaultWidth, quality }) : finalSrc
    return
  }

  if (useResponsivePreload) {
    applyResponsivePreload({
      link,
      finalSrc,
      defaultWidth,
      quality,
      preloadSizes,
      loader,
      preloadFormat,
      preloadAvifOnly,
    })
    return
  }

  link.href = resolveLoaderOrBuiltUrl(loader, finalSrc, defaultWidth, quality)
  if (preloadSizes != null) link.setAttribute('imagesizes', preloadSizes)
}

function pickExplicitOrIntrinsic(
  explicit: number | undefined,
  fill: boolean,
  intrinsic: number | undefined,
): number | undefined {
  if (explicit != null && explicit !== 0) return explicit
  if (!fill && intrinsic != null && intrinsic !== 0) return intrinsic
  return undefined
}

function resolveImageDimensions(options: {
  readonly src: string | StaticImageData
  readonly width: number | undefined
  readonly height: number | undefined
  readonly fill: boolean
  readonly placeholder: 'blur' | 'empty'
  readonly blurDataURL: string | undefined
  readonly loader: ImageProps['loader']
}): {
  readonly imgSrc: string
  readonly imgWidth: number | undefined
  readonly imgHeight: number | undefined
  readonly inlineBlurDataURL: string | undefined
  readonly optimizerBlurUrl: string | undefined
} {
  const { src, width, height, fill, placeholder, blurDataURL, loader } = options
  const imgSrc = typeof src === 'string' ? src : src.src
  const intrinsicWidth = typeof src !== 'string' ? src.width : undefined
  const intrinsicHeight = typeof src !== 'string' ? src.height : undefined
  const explicitBlur =
    blurDataURL != null && blurDataURL !== ''
      ? blurDataURL
      : typeof src !== 'string'
        ? src.blurDataURL
        : undefined
  const inlineBlurDataURL = explicitBlur != null && explicitBlur !== '' ? explicitBlur : undefined
  const optimizerBlurUrl =
    placeholder === 'blur' && inlineBlurDataURL == null
      ? resolveLoaderOrBuiltUrl(
          loader,
          imgSrc,
          BLUR_PLACEHOLDER_WIDTH,
          BLUR_PLACEHOLDER_QUALITY,
          'jpeg',
        )
      : undefined
  return {
    imgSrc,
    imgWidth: pickExplicitOrIntrinsic(width, fill, intrinsicWidth),
    imgHeight: pickExplicitOrIntrinsic(height, fill, intrinsicHeight),
    inlineBlurDataURL,
    optimizerBlurUrl,
  }
}

function fillImageStyle(style: React.CSSProperties | undefined): React.CSSProperties {
  return {
    position: 'absolute',
    inset: 0,
    width: '100%',
    height: '100%',
    objectFit: style?.objectFit ?? 'cover',
  }
}

function blurPendingStyle(imgBlurDataURL: string): React.CSSProperties {
  return {
    backgroundImage: `url("${imgBlurDataURL}")`,
    backgroundSize: 'cover',
    backgroundPosition: 'center',
    filter: 'blur(20px)',
    transition: 'filter 0.3s ease-out',
  }
}

function buildImageStyle(options: {
  readonly style: React.CSSProperties | undefined
  readonly fill: boolean
  readonly placeholder: 'blur' | 'empty'
  readonly imgBlurDataURL: string | undefined
  readonly blurComplete: boolean
}): React.CSSProperties {
  const { style, fill, placeholder, imgBlurDataURL, blurComplete } = options
  const blurPending =
    placeholder === 'blur' && imgBlurDataURL != null && imgBlurDataURL !== '' && !blurComplete
  const blurDone = placeholder === 'blur' && blurComplete
  return {
    ...style,
    ...(fill ? fillImageStyle(style) : null),
    ...(blurPending ? blurPendingStyle(imgBlurDataURL) : null),
    ...(blurDone ? { filter: 'none', transition: 'filter 0.3s ease-out' } : null),
  }
}

function resolveImageSrc(
  loader: ImageProps['loader'],
  finalSrc: string,
  width: number,
  quality: number,
): string {
  return loader != null ? loader({ src: finalSrc, width, quality }) : finalSrc
}

function buildOptimizedSrcSet(
  loader: ImageProps['loader'],
  finalSrc: string,
  widths: readonly number[],
  quality: number,
  format?: ImageFormat,
): string {
  if (loader != null)
    return widths.map(w => `${loader({ src: finalSrc, width: w, quality })} ${w}w`).join(', ')
  return widths.map(w => `${buildImageUrl(finalSrc, w, quality, format)} ${w}w`).join(', ')
}

function resolvePictureSizes(
  sizes: string | undefined,
  shouldUseSrcSet: boolean,
): string | undefined {
  if (sizes != null && sizes !== '') return sizes
  return shouldUseSrcSet ? '100vw' : undefined
}

interface ImageElementOptions {
  readonly imgRef: React.RefObject<HTMLImageElement | null>
  readonly src: string
  readonly srcSet?: string
  readonly sizes?: string
  readonly alt: string
  readonly showAltText: boolean
  readonly fill: boolean
  readonly imgWidth: number | undefined
  readonly imgHeight: number | undefined
  readonly shouldPreload: boolean
  readonly loading: 'lazy' | 'eager'
  readonly imgDecoding: 'async' | 'sync' | 'auto'
  readonly placeholder: 'blur' | 'empty'
  readonly onLoad: ((event: React.SyntheticEvent<HTMLImageElement>) => void) | undefined
  readonly handleLoad: (event: React.SyntheticEvent<HTMLImageElement>) => void
  readonly handleError: (event: React.SyntheticEvent<HTMLImageElement>) => void
  readonly imgStyle: React.CSSProperties
  readonly className: string | undefined
}

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
function renderImageElement({
  imgRef,
  src,
  srcSet,
  sizes,
  alt,
  showAltText,
  fill,
  imgWidth,
  imgHeight,
  shouldPreload,
  loading,
  imgDecoding,
  placeholder,
  onLoad,
  handleLoad,
  handleError,
  imgStyle,
  className,
}: ImageElementOptions): React.ReactElement {
  return (
    <img
      ref={imgRef}
      src={src}
      srcSet={srcSet}
      sizes={sizes}
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
  const { imgSrc, imgWidth, imgHeight, inlineBlurDataURL, optimizerBlurUrl } =
    resolveImageDimensions({
      src,
      width,
      height,
      fill,
      placeholder,
      blurDataURL,
      loader,
    })
  const finalSrc = overrideSrc != null && overrideSrc !== '' ? overrideSrc : imgSrc
  const shouldPreload = preload
  const imgDecoding = decoding ?? (preload ? 'sync' : 'async')
  const sizePlan = resolveOptimizedSizePlan({
    fill,
    width,
    intrinsicWidth: typeof src !== 'string' ? src.width : undefined,
  })
  const shouldUseSrcSet = sizePlan.widths.length > 1 || sizePlan.widths[0] !== sizePlan.defaultWidth
  const shouldEagerBlur = shouldPreload || loading === 'eager'

  const [blurComplete, setBlurComplete] = useState(false)
  const [showAltText, setShowAltText] = useState(false)
  const [blurSrcKey, setBlurSrcKey] = useState(finalSrc)
  const [nearViewport, setNearViewport] = useState(shouldEagerBlur)
  const imgRef = useRef<HTMLImageElement>(null)
  const onLoadRef = useRef(onLoad)
  const pictureRef = useRef<HTMLPictureElement>(null)

  if (blurSrcKey !== finalSrc) {
    setBlurSrcKey(finalSrc)
    setNearViewport(shouldEagerBlur)
    setBlurComplete(false)
  }

  const activeBlurUrl =
    inlineBlurDataURL ?? (shouldEagerBlur || nearViewport ? optimizerBlurUrl : undefined)

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
    configureImagePreloadLink({
      link,
      finalSrc,
      defaultWidth: sizePlan.defaultWidth,
      quality,
      sizes,
      loader,
      unoptimized,
      fill,
      shouldUseSrcSet,
    })

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
    if (shouldEagerBlur || nearViewport || optimizerBlurUrl == null || placeholder !== 'blur') {
      return undefined
    }

    const img = imgRef.current
    if (!img) return undefined

    const observer = new IntersectionObserver(
      entries => {
        if (entries.some(entry => entry.isIntersecting)) {
          setNearViewport(true)
          observer.disconnect()
        }
      },
      {
        rootMargin: '200px',
      },
    )

    observer.observe(img)

    return () => {
      observer.disconnect()
    }
  }, [shouldEagerBlur, nearViewport, optimizerBlurUrl, placeholder, finalSrc])

  const imgStyle = buildImageStyle({
    style,
    fill,
    placeholder,
    imgBlurDataURL: activeBlurUrl,
    blurComplete,
  })

  const sharedElementOptions = {
    imgRef,
    alt,
    showAltText,
    fill,
    imgWidth,
    imgHeight,
    shouldPreload,
    loading,
    imgDecoding,
    placeholder,
    onLoad,
    handleLoad,
    handleError,
    imgStyle,
    className,
  }

  if (unoptimized) {
    // oxlint-disable-next-line react/refs
    return renderImageElement({
      ...sharedElementOptions,
      src: resolveImageSrc(loader, finalSrc, sizePlan.defaultWidth, quality),
    })
  }

  const resolvedSizes = resolvePictureSizes(sizes, shouldUseSrcSet)
  const mainSrc = resolveLoaderOrBuiltUrl(loader, finalSrc, sizePlan.defaultWidth, quality)
  // oxlint-disable-next-line react/refs
  const imgElement = renderImageElement({
    ...sharedElementOptions,
    src: mainSrc,
    srcSet: shouldUseSrcSet
      ? buildOptimizedSrcSet(loader, finalSrc, sizePlan.widths, quality)
      : undefined,
    sizes: shouldUseSrcSet ? resolvedSizes : undefined,
  })

  if (!shouldUseSrcSet) return imgElement

  return (
    <picture ref={pictureRef}>
      {DEFAULT_FORMATS.includes('avif') && (
        <source
          type="image/avif"
          srcSet={buildOptimizedSrcSet(loader, finalSrc, sizePlan.widths, quality, 'avif')}
          sizes={resolvedSizes}
        />
      )}
      {DEFAULT_FORMATS.includes('webp') && (
        <source
          type="image/webp"
          srcSet={buildOptimizedSrcSet(loader, finalSrc, sizePlan.widths, quality, 'webp')}
          sizes={resolvedSizes}
        />
      )}
      {imgElement}
    </picture>
  )
}
