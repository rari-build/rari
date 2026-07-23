import {
  ActionDidNotRevalidate,
  ActionDidRevalidateDynamicOnly,
  ActionDidRevalidateStaticAndDynamic,
  parseActionRevalidationKind,
} from '@rari/runtime/actions/revalidation-kind'
import { describe, expect, it } from 'vite-plus/test'

describe('parseActionRevalidationKind', () => {
  it('parses Next-style numeric JSON values', () => {
    expect(parseActionRevalidationKind('2')).toBe(ActionDidRevalidateDynamicOnly)
    expect(parseActionRevalidationKind('1')).toBe(ActionDidRevalidateStaticAndDynamic)
  })

  it('returns not-revalidated for missing or unknown values', () => {
    expect(parseActionRevalidationKind(null)).toBe(ActionDidNotRevalidate)
    expect(parseActionRevalidationKind('unknown')).toBe(ActionDidNotRevalidate)
    expect(parseActionRevalidationKind('/actions')).toBe(ActionDidNotRevalidate)
  })
})
