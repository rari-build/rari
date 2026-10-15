import { afterEach, describe, expect, it } from 'vite-plus/test'
import {
  DEFAULT_DEVICE_SIZES,
  DEFAULT_IMAGE_SIZES,
} from '../../../packages/rari/src/image/constants'
import { configureImageSizes } from '../../../packages/rari/src/image/runtime-config'
import { resolveOptimizedSizePlan } from '../../../packages/rari/src/image/size-plan'

const ALL_SIZES = [...DEFAULT_IMAGE_SIZES, ...DEFAULT_DEVICE_SIZES].sort((a, b) => a - b)

afterEach(() => {
  configureImageSizes({
    deviceSizes: DEFAULT_DEVICE_SIZES,
    imageSizes: DEFAULT_IMAGE_SIZES,
  })
})

describe('resolveOptimizedSizePlan', () => {
  it('uses device sizes for fill mode instead of intrinsic width', () => {
    const plan = resolveOptimizedSizePlan({
      fill: true,
      intrinsicWidth: 4096,
    })

    expect(plan.defaultWidth).toBe(Math.max(...DEFAULT_DEVICE_SIZES))
    expect(plan.widths).toEqual(DEFAULT_DEVICE_SIZES)
    expect(plan.widths).not.toContain(4096)
  })

  it('emits 1x and 2x candidates for fixed layouts without sizes', () => {
    expect(resolveOptimizedSizePlan({ fill: false, width: 600 })).toEqual({
      defaultWidth: 640,
      widths: [640, 1200],
    })
  })

  it('snaps oversized intrinsic widths to configured sizes for fixed layouts', () => {
    expect(
      resolveOptimizedSizePlan({
        fill: false,
        intrinsicWidth: 4096,
      }),
    ).toEqual({
      defaultWidth: 3840,
      widths: [3840],
    })
  })

  it('uses the full size list when sizes has no vw (so the browser can pick)', () => {
    const plan = resolveOptimizedSizePlan({
      fill: false,
      width: 640,
      sizes: '240px',
    })

    expect(plan.defaultWidth).toBe(640)
    expect(plan.widths).toEqual(ALL_SIZES)
  })

  it('filters candidates by smallest vw ratio when sizes includes vw', () => {
    const plan = resolveOptimizedSizePlan({
      fill: false,
      width: 1200,
      sizes: '(max-width: 768px) 100vw, 50vw',
    })
    const minWidth = DEFAULT_DEVICE_SIZES[0] * 0.5

    expect(plan.defaultWidth).toBe(1200)
    expect(plan.widths).toEqual(ALL_SIZES.filter(size => size >= minWidth))
    expect(plan.widths[0]).toBeGreaterThanOrEqual(minWidth)
  })

  it('parses comma-delimited vw values without spaces', () => {
    const plan = resolveOptimizedSizePlan({
      fill: false,
      width: 1200,
      sizes: '100vw,25vw',
    })
    const minWidth = DEFAULT_DEVICE_SIZES[0] * 0.25

    expect(plan.widths).toEqual(ALL_SIZES.filter(size => size >= minWidth))
    expect(plan.widths[0]).toBeGreaterThanOrEqual(minWidth)
  })

  it('uses vite-configured deviceSizes and imageSizes when set', () => {
    configureImageSizes({
      deviceSizes: [640, 750, 828, 1080, 1200, 1600],
      imageSizes: [64, 96, 128, 160, 208, 256, 320, 384, 416, 480, 560],
    })

    const fixed = resolveOptimizedSizePlan({ fill: false, width: 416 })
    expect(fixed).toEqual({
      defaultWidth: 416,
      widths: [416, 1080],
    })

    const withSizes = resolveOptimizedSizePlan({
      fill: false,
      width: 640,
      sizes: '240px',
    })
    expect(withSizes.widths).toContain(208)
    expect(withSizes.widths).toContain(416)
    expect(withSizes.widths).toContain(1600)
    expect(withSizes.widths).not.toContain(1920)
    expect(withSizes.widths).not.toContain(16)

    const fill = resolveOptimizedSizePlan({ fill: true })
    expect(fill.widths).toEqual([640, 750, 828, 1080, 1200, 1600])
    expect(fill.defaultWidth).toBe(1600)
  })
})
