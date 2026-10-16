import {
  getAllImageSizes,
  getImageDeviceSizes,
  getMaxOptimizedWidth,
} from 'rari/image/runtime-config'

function clampOptimizedWidth(width: number): number {
  return Math.min(Math.max(width, 1), getMaxOptimizedWidth())
}

function nearestOptimizedSize(target: number): number {
  const allSizes = getAllImageSizes()
  const clamped = clampOptimizedWidth(target)
  return allSizes.find(size => size >= clamped) ?? allSizes[allSizes.length - 1]
}

function widthsForSizesProp(sizes: string): number[] {
  const viewportWidthRe = /(^|[,\s])(1?\d?\d)vw/g
  const percentSizes: number[] = []
  for (
    let match = viewportWidthRe.exec(sizes);
    match != null;
    match = viewportWidthRe.exec(sizes)
  ) {
    percentSizes.push(Number.parseInt(match[2], 10))
  }

  const allSizes = getAllImageSizes()
  if (percentSizes.length === 0) {
    return [...allSizes]
  }

  const smallestRatio = Math.min(...percentSizes) * 0.01
  const minWidth = getImageDeviceSizes()[0] * smallestRatio
  return allSizes.filter(size => size >= minWidth)
}

export function resolveOptimizedSizePlan(options: {
  readonly fill: boolean
  readonly width?: number
  readonly intrinsicWidth?: number
  readonly sizes?: string
}): {
  readonly defaultWidth: number
  readonly widths: readonly number[]
} {
  const { fill, width, intrinsicWidth, sizes } = options
  const deviceSizes = getImageDeviceSizes()

  if (fill) {
    return {
      defaultWidth: getMaxOptimizedWidth(),
      widths: deviceSizes,
    }
  }

  const resolved =
    width != null && width !== 0
      ? width
      : intrinsicWidth != null && intrinsicWidth !== 0
        ? intrinsicWidth
        : undefined

  if (sizes != null && sizes !== '') {
    const widths = widthsForSizesProp(sizes)
    return {
      defaultWidth: resolved != null ? nearestOptimizedSize(resolved) : (widths.at(-1) ?? 1920),
      widths,
    }
  }

  if (resolved == null) {
    return {
      defaultWidth: 1920,
      widths: deviceSizes,
    }
  }

  const widths = [...new Set([resolved, resolved * 2].map(value => nearestOptimizedSize(value)))]
  widths.sort((a, b) => a - b)

  return {
    defaultWidth: widths[0],
    widths,
  }
}
