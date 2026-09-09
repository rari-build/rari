import { describe, expect, it } from 'vite-plus/test'
import { DEFAULT_DEVICE_SIZES } from '../../../packages/rari/src/image/constants'
import { resolveOptimizedSizePlan } from '../../../packages/rari/src/image/size-plan'

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

  it('keeps an explicit width for fixed layouts', () => {
    expect(resolveOptimizedSizePlan({ fill: false, width: 600 })).toEqual({
      defaultWidth: 600,
      widths: [600],
    })
  })

  it('clamps oversized intrinsic widths for fixed layouts', () => {
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
})
