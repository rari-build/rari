import type { Robots } from 'rari'

export default function robots(): Robots {
  return {
    rules: {
      userAgent: '*',
      allow: '/',
    },
  }
}
