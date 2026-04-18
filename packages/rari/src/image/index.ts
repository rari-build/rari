import { Image as ImageComponent } from './Image'

export type { ImageFormat } from './constants'
export {
  DEFAULT_DEVICE_SIZES,
  DEFAULT_FORMATS,
  DEFAULT_IMAGE_SIZES,
  DEFAULT_MAX_CACHE_SIZE,
  DEFAULT_MINIMUM_CACHE_TTL,
  DEFAULT_QUALITY_LEVELS,
} from './constants'
export type { ImageProps, StaticImageData } from './Image'

const isServer = typeof window === 'undefined'
const Image: any = ImageComponent

if (isServer) {
  Image.$$typeof = Symbol.for('react.client.reference')
  Image.$$id = 'rari/image#Image'
}

export { Image }
