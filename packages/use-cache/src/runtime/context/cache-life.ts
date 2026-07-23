export interface CacheLifeProfile {
  readonly stale?: number
  readonly revalidate?: number
  readonly expire?: number
}

export type CacheLifeProfileName =
  | 'default'
  | 'seconds'
  | 'minutes'
  | 'hours'
  | 'days'
  | 'weeks'
  | 'max'

const DEFAULT_PROFILE: Required<Pick<CacheLifeProfile, 'stale' | 'revalidate'>> & {
  expire?: number
} = {
  stale: 300,
  revalidate: 900,
  expire: undefined,
}

export const CACHE_LIFE_PROFILES: Record<CacheLifeProfileName, CacheLifeProfile> = {
  default: DEFAULT_PROFILE,
  seconds: { stale: 30, revalidate: 1, expire: 60 },
  minutes: { stale: 300, revalidate: 60, expire: 3600 },
  hours: { stale: 300, revalidate: 3600, expire: 86_400 },
  days: { stale: 300, revalidate: 86_400, expire: 604_800 },
  weeks: { stale: 300, revalidate: 604_800, expire: 2_592_000 },
  max: { stale: 300, revalidate: 2_592_000, expire: 31_536_000 },
}

const ONE_YEAR_MS = 365 * 24 * 60 * 60 * 1000

function resolveProfile(profile: CacheLifeProfileName | CacheLifeProfile): CacheLifeProfile {
  if (typeof profile === 'string') return { ...CACHE_LIFE_PROFILES[profile] }

  return {
    stale: profile.stale ?? DEFAULT_PROFILE.stale,
    revalidate: profile.revalidate ?? DEFAULT_PROFILE.revalidate,
    expire: profile.expire ?? DEFAULT_PROFILE.expire,
  }
}

export function cacheLifeToTtlMs(profile: CacheLifeProfile | undefined): number {
  if (!profile) return DEFAULT_PROFILE.revalidate * 1000

  const expire = profile.expire
  if (expire !== undefined && expire > 0) return expire * 1000

  if (profile.revalidate !== undefined) return profile.revalidate * 1000

  const revalidate = DEFAULT_PROFILE.revalidate
  if (revalidate > 0) return revalidate * 1000

  return ONE_YEAR_MS
}

export function normalizeCacheLife(
  profile: CacheLifeProfileName | CacheLifeProfile,
): CacheLifeProfile {
  const resolved = resolveProfile(profile)

  if (
    resolved.revalidate !== undefined &&
    resolved.expire !== undefined &&
    resolved.expire > 0 &&
    resolved.revalidate > 0 &&
    resolved.expire <= resolved.revalidate
  ) {
    throw new Error('[rari] cacheLife: expire must be longer than revalidate when both are set.')
  }

  return resolved
}
