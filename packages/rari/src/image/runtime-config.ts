import { DEFAULT_DEVICE_SIZES, DEFAULT_IMAGE_SIZES } from './constants'

function sortedUnique(sizes: readonly number[]): readonly number[] {
  return Object.freeze(
    [...new Set(sizes.filter(size => Number.isFinite(size) && size > 0))].sort((a, b) => a - b),
  )
}

let deviceSizes: readonly number[] = DEFAULT_DEVICE_SIZES
let imageSizes: readonly number[] = DEFAULT_IMAGE_SIZES
let allSizes: readonly number[] = sortedUnique([...DEFAULT_IMAGE_SIZES, ...DEFAULT_DEVICE_SIZES])
let maxOptimizedWidth = Math.max(...DEFAULT_DEVICE_SIZES)

export function configureImageSizes(options: {
  readonly deviceSizes?: readonly number[]
  readonly imageSizes?: readonly number[]
}): void {
  if (options.deviceSizes != null && options.deviceSizes.length > 0) {
    const nextDeviceSizes = sortedUnique(options.deviceSizes)
    if (nextDeviceSizes.length > 0) {
      deviceSizes = nextDeviceSizes
    }
  }
  if (options.imageSizes != null && options.imageSizes.length > 0) {
    const nextImageSizes = sortedUnique(options.imageSizes)
    if (nextImageSizes.length > 0) {
      imageSizes = nextImageSizes
    }
  }
  allSizes = sortedUnique([...imageSizes, ...deviceSizes])
  maxOptimizedWidth = Math.max(...deviceSizes)
}

export function getImageDeviceSizes(): readonly number[] {
  return deviceSizes
}

export function getImageSizes(): readonly number[] {
  return imageSizes
}

export function getAllImageSizes(): readonly number[] {
  return allSizes
}

export function getMaxOptimizedWidth(): number {
  return maxOptimizedWidth
}
