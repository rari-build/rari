import { generateOGImage } from '@/lib/site'

export default function Image() {
  return generateOGImage({
    title: 'Runtime Accelerated Rendering Infrastructure',
    description: 'Performance-first React framework powered by Rust',
    logoSize: 'large',
  })
}
