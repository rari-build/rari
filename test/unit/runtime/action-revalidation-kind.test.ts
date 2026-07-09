import {
  ActionDidNotRevalidate,
  ActionDidRevalidateDynamicOnly,
  ActionDidRevalidateStaticAndDynamic,
  parseActionRevalidationKind,
} from '@rari/runtime/action-revalidation-kind'
import { describe, expect, it } from 'vite-plus/test'

describe('parseActionRevalidationKind', () => {
  it('parses Next-style numeric JSON values', () => {
    expect(parseActionRevalidationKind('2')).toBe(ActionDidRevalidateDynamicOnly)
    expect(parseActionRevalidationKind('1')).toBe(ActionDidRevalidateStaticAndDynamic)
  })

  it('treats legacy pathname headers as dynamic-only refresh', () => {
    expect(parseActionRevalidationKind('/actions')).toBe(ActionDidRevalidateDynamicOnly)
  })

  it('returns not-revalidated for missing or unknown values', () => {
    expect(parseActionRevalidationKind(null)).toBe(ActionDidNotRevalidate)
    expect(parseActionRevalidationKind('unknown')).toBe(ActionDidNotRevalidate)
  })
})
