export const URL_PATTERNS = {
  HOME: /\/$/,
  ABOUT: /\/about$/,
  NESTED: /\/nested$/,
  NESTED_DEEP: /\/nested\/deep$/,
  BLOG: /\/blog$/,
  BLOG_POST: /\/blog\/[^/]+$/,
  PRODUCTS: /\/products$/,
  PRODUCT_DETAIL: /\/products\/[^/]+\/[^/]+$/,
  DOCS: /\/docs\/.+/,
  SHOP: /\/shop/,
} as const
