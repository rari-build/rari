import { DEFAULT_DEVICE_SIZES } from './constants'

const MAX_OPTIMIZED_WIDTH = Math.max(...DEFAULT_DEVICE_SIZES)

function clampOptimizedWidth(width: number): number {
  return Math.min(Math.max(width, 1), MAX_OPTIMIZED_WIDTH)
}

export function resolveOptimizedSizePlan(options: {
  readonly fill: boolean
  readonly width?: number
  readonly intrinsicWidth?: number
}): {
  readonly defaultWidth: number
  readonly widths: readonly number[]
} {
  const { fill, width, intrinsicWidth } = options

  if (fill) {
    return {
      defaultWidth: MAX_OPTIMIZED_WIDTH,
      widths: DEFAULT_DEVICE_SIZES,
    }
  }

  const resolved =
    width != null && width !== 0
      ? width
      : intrinsicWidth != null && intrinsicWidth !== 0
        ? intrinsicWidth
        : undefined

  if (resolved == null) {
    return {
      defaultWidth: 1920,
      widths: DEFAULT_DEVICE_SIZES,
    }
  }

  const clamped = clampOptimizedWidth(resolved)
  return {
    defaultWidth: clamped,
    widths: [clamped],
  }
}

export { MAX_OPTIMIZED_WIDTH }
