'use client'

import type { CSSProperties, ReactElement, RefObject, SyntheticEvent } from 'react'
import type { ImageFormat } from './constants'
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { preload as preloadResource } from 'react-dom'
import { BLUR_PLACEHOLDER_QUALITY, BLUR_PLACEHOLDER_WIDTH, DEFAULT_FORMATS } from './constants'
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
  readonly style?: CSSProperties
  readonly className?: string
  readonly onLoad?: (event: SyntheticEvent<HTMLImageElement>) => void
  readonly onError?: (event: SyntheticEvent<HTMLImageElement>) => void
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

function resolveDefaultSizes(
  sizes: string | undefined,
  useResponsive: boolean,
  fill = false,
): string | undefined {
  if (sizes != null && sizes !== '') return sizes
  return useResponsive || fill ? '100vw' : undefined
}

function primaryOptimizerFormat(loader: ImageProps['loader']): ImageFormat | undefined {
  if (loader != null) return undefined
  if (DEFAULT_FORMATS.length === 1 && DEFAULT_FORMATS[0] === 'avif') return 'avif'
  return undefined
}

function preloadImageResource(options: {
  readonly enabled: boolean
  readonly finalSrc: string
  readonly defaultWidth: number
  readonly widths: readonly number[]
  readonly quality: number
  readonly sizes: string | undefined
  readonly loader: ImageProps['loader']
  readonly unoptimized: boolean
  readonly fill: boolean
  readonly shouldUseSrcSet: boolean
  readonly format: ImageFormat | undefined
}): void {
  if (!options.enabled) return

  const {
    finalSrc,
    defaultWidth,
    widths,
    quality,
    sizes,
    loader,
    unoptimized,
    fill,
    shouldUseSrcSet,
    format,
  } = options

  if (unoptimized) {
    preloadResource(
      loader != null ? loader({ src: finalSrc, width: defaultWidth, quality }) : finalSrc,
      { as: 'image' },
    )
    return
  }

  const href = resolveLoaderOrBuiltUrl(loader, finalSrc, defaultWidth, quality, format)
  const useResponsivePreload = shouldUseSrcSet
  const preloadSizes = resolveDefaultSizes(sizes, useResponsivePreload, fill)

  if (useResponsivePreload) {
    preloadResource(href, {
      as: 'image',
      imageSrcSet: buildOptimizedSrcSet(loader, finalSrc, widths, quality, format),
      ...(preloadSizes != null ? { imageSizes: preloadSizes } : {}),
      ...(format === 'avif' ? { type: 'image/avif' } : {}),
    })
    return
  }

  preloadResource(href, {
    as: 'image',
    ...(format === 'avif' ? { type: 'image/avif' } : {}),
  })
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

function fillImageStyle(style: CSSProperties | undefined): CSSProperties {
  return {
    position: 'absolute',
    inset: 0,
    width: '100%',
    height: '100%',
    objectFit: style?.objectFit ?? 'cover',
  }
}

function blurPendingStyle(imgBlurDataURL: string): CSSProperties {
  return {
    backgroundImage: `url("${imgBlurDataURL}")`,
    backgroundSize: 'cover',
    backgroundPosition: 'center',
    filter: 'blur(20px)',
    transition: 'filter 0.3s ease-out',
  }
}

function isImageDecoded(img: Readonly<{ complete: boolean; naturalWidth: number }>): boolean {
  return img.complete && img.naturalWidth > 0
}

function buildImageStyle(options: {
  readonly style: CSSProperties | undefined
  readonly fill: boolean
  readonly placeholder: 'blur' | 'empty'
  readonly imgBlurDataURL: string | undefined
  readonly blurComplete: boolean
}): CSSProperties {
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

function buildOptimizedSrcSet(
  loader: ImageProps['loader'],
  finalSrc: string,
  widths: readonly number[],
  quality: number,
  format?: ImageFormat,
): string {
  return widths
    .map(w => `${resolveLoaderOrBuiltUrl(loader, finalSrc, w, quality, format)} ${w}w`)
    .join(', ')
}

function resolveDisplaySrc(options: {
  readonly unoptimized: boolean
  readonly loader: ImageProps['loader']
  readonly finalSrc: string
  readonly width: number
  readonly quality: number
  readonly format?: ImageFormat
}): string {
  const { unoptimized, loader, finalSrc, width, quality, format } = options
  if (!unoptimized) return resolveLoaderOrBuiltUrl(loader, finalSrc, width, quality, format)
  return loader != null ? loader({ src: finalSrc, width, quality }) : finalSrc
}

function observeNearViewport(
  img: HTMLElement | null,
  enabled: boolean,
  onNear: () => void,
): (() => void) | undefined {
  if (!enabled || img == null) return undefined

  const observer = new IntersectionObserver(
    entries => {
      if (entries.some(entry => entry.isIntersecting)) {
        onNear()
        observer.disconnect()
      }
    },
    { rootMargin: '200px' },
  )
  observer.observe(img)
  return () => {
    observer.disconnect()
  }
}

function pictureWithFormatSources(options: {
  readonly imgElement: ReactElement
  readonly loader: ImageProps['loader']
  readonly finalSrc: string
  readonly widths: readonly number[]
  readonly quality: number
  readonly sizes: string | undefined
}): ReactElement {
  const { imgElement, loader, finalSrc, widths, quality, sizes } = options
  return (
    <picture>
      {DEFAULT_FORMATS.includes('avif') && (
        <source
          type="image/avif"
          srcSet={buildOptimizedSrcSet(loader, finalSrc, widths, quality, 'avif')}
          sizes={sizes}
        />
      )}
      {DEFAULT_FORMATS.includes('webp') && (
        <source
          type="image/webp"
          srcSet={buildOptimizedSrcSet(loader, finalSrc, widths, quality, 'webp')}
          sizes={sizes}
        />
      )}
      {imgElement}
    </picture>
  )
}

function useImageBlurState(options: {
  readonly finalSrc: string
  readonly shouldEagerBlur: boolean
  readonly inlineBlurDataURL: string | undefined
  readonly optimizerBlurUrl: string | undefined
  readonly placeholder: 'blur' | 'empty'
}): {
  readonly imgRef: RefObject<HTMLImageElement | null>
  readonly blurComplete: boolean
  readonly setBlurComplete: (complete: boolean) => void
  readonly activeBlurUrl: string | undefined
} {
  const { finalSrc, shouldEagerBlur, inlineBlurDataURL, optimizerBlurUrl, placeholder } = options
  const [blurComplete, setBlurComplete] = useState(false)
  const [blurSrcKey, setBlurSrcKey] = useState(finalSrc)
  const [nearViewport, setNearViewport] = useState(shouldEagerBlur)
  const imgRef = useRef<HTMLImageElement>(null)

  if (blurSrcKey !== finalSrc) {
    setBlurSrcKey(finalSrc)
    setNearViewport(shouldEagerBlur)
    setBlurComplete(false)
  }

  useLayoutEffect(() => {
    if (placeholder !== 'blur') return
    const img = imgRef.current
    if (img != null && isImageDecoded(img)) setBlurComplete(true)
  }, [placeholder, finalSrc])

  useEffect(
    () =>
      observeNearViewport(
        imgRef.current,
        !shouldEagerBlur && !nearViewport && optimizerBlurUrl != null && placeholder === 'blur',
        () => {
          setNearViewport(true)
        },
      ),
    [shouldEagerBlur, nearViewport, optimizerBlurUrl, placeholder, finalSrc],
  )

  return {
    imgRef,
    blurComplete,
    setBlurComplete,
    activeBlurUrl:
      inlineBlurDataURL ?? (shouldEagerBlur || nearViewport ? optimizerBlurUrl : undefined),
  }
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
  const imgDecoding = decoding ?? (preload ? 'sync' : 'async')
  const sizePlan = resolveOptimizedSizePlan({
    fill,
    width,
    intrinsicWidth: typeof src !== 'string' ? src.width : undefined,
  })
  const shouldUseSrcSet = sizePlan.widths.length > 1 || sizePlan.widths[0] !== sizePlan.defaultWidth
  const shouldEagerBlur = preload || loading === 'eager'
  const optimizerFormat = primaryOptimizerFormat(loader)

  const [showAltText, setShowAltText] = useState(false)
  const onLoadRef = useRef(onLoad)
  const { imgRef, blurComplete, setBlurComplete, activeBlurUrl } = useImageBlurState({
    finalSrc,
    shouldEagerBlur,
    inlineBlurDataURL,
    optimizerBlurUrl,
    placeholder,
  })

  useEffect(() => {
    onLoadRef.current = onLoad
  }, [onLoad])

  const handleLoad = useCallback(
    (event: SyntheticEvent<HTMLImageElement>) => {
      const img = event.currentTarget
      const loadedSrc = img.currentSrc !== '' ? img.currentSrc : img.src

      if (loadedSrc !== '' && img.complete) {
        if (placeholder === 'blur') setBlurComplete(true)

        if (onLoadRef.current) onLoadRef.current(event)
      }
    },
    [placeholder, setBlurComplete],
  )

  const handleError = useCallback(
    (event: SyntheticEvent<HTMLImageElement>) => {
      setShowAltText(true)
      if (placeholder === 'blur') setBlurComplete(true)

      if (onError) onError(event)
    },
    [placeholder, onError, setBlurComplete],
  )

  const imgStyle = buildImageStyle({
    style,
    fill,
    placeholder,
    imgBlurDataURL: activeBlurUrl,
    blurComplete,
  })

  const resolvedSrc = resolveDisplaySrc({
    unoptimized,
    loader,
    finalSrc,
    width: sizePlan.defaultWidth,
    quality,
    format: optimizerFormat,
  })
  const resolvedSizes = resolveDefaultSizes(sizes, shouldUseSrcSet, fill)
  const srcSet =
    !unoptimized && shouldUseSrcSet
      ? buildOptimizedSrcSet(loader, finalSrc, sizePlan.widths, quality, optimizerFormat)
      : undefined

  preloadImageResource({
    enabled: preload,
    finalSrc,
    defaultWidth: sizePlan.defaultWidth,
    widths: sizePlan.widths,
    quality,
    sizes: resolvedSizes,
    loader,
    unoptimized,
    fill,
    shouldUseSrcSet,
    format: optimizerFormat,
  })

  const imgElement = (
    <img
      ref={imgRef}
      src={resolvedSrc}
      srcSet={srcSet}
      sizes={srcSet != null ? resolvedSizes : undefined}
      alt={showAltText ? alt : ''}
      width={fill ? undefined : imgWidth}
      height={fill ? undefined : imgHeight}
      loading={preload ? 'eager' : loading}
      fetchPriority={preload ? 'high' : 'auto'}
      decoding={imgDecoding}
      onLoad={placeholder === 'blur' || onLoad != null ? handleLoad : undefined}
      onError={handleError}
      style={imgStyle}
      className={className}
    />
  )

  if (unoptimized || !shouldUseSrcSet) return imgElement

  return pictureWithFormatSources({
    imgElement,
    loader,
    finalSrc,
    widths: sizePlan.widths,
    quality,
    sizes: resolvedSizes,
  })
}
