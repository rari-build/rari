export type ActionRevalidationKind = 0 | 1 | 2

export const ActionDidNotRevalidate = 0 as const
export const ActionDidRevalidateStaticAndDynamic = 1 as const
export const ActionDidRevalidateDynamicOnly = 2 as const

export function parseActionRevalidationKind(header: string | null): ActionRevalidationKind {
  if (header == null || header === '') return ActionDidNotRevalidate

  try {
    const parsed = JSON.parse(header) as unknown
    if (
      parsed === ActionDidRevalidateStaticAndDynamic ||
      parsed === ActionDidRevalidateDynamicOnly
    ) {
      return parsed
    }
  } catch {
    // Non-JSON headers are ignored.
  }

  return ActionDidNotRevalidate
}
