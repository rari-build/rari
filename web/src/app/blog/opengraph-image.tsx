import { generateOGImage } from '@/lib/og-image'

export default function Image() {
  return generateOGImage({
    title: 'Blog',
    description: 'Latest news, updates, and insights',
    logoSize: 'large',
  })
}
