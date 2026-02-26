export const URL_PATTERNS = {
  HOME: /\/$/,
  DOCS_PATH_REGEX: /\/docs(?:\/|$)/,
  DOCS_GETTING_STARTED: /\/docs\/getting-started$/,
  DOCS_API_REFERENCE: /\/docs\/api-reference$/,
  DOCS_ROUTING: /\/docs\/getting-started\/routing$/,
  DOCS_DATA_FETCHING: /\/docs\/getting-started\/data-fetching$/,
  BLOG: /\/blog$/,
  ENTERPRISE: /\/enterprise$/,
} as const

export const MOBILE_DEVICES = {
  IPHONE: {
    userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
    viewport: { width: 375, height: 667 },
  },
  ANDROID: {
    userAgent: 'Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36',
    viewport: { width: 412, height: 915 },
  },
} as const
