export function formatCompactNumber(num: number): string {
  if (num < 1000) {
    return num.toString()
  }

  if (num < 1_000_000) {
    const thousands = num / 1000
    const rounded = Math.round(thousands * 10) / 10
    return rounded % 1 === 0
      ? `${Math.round(rounded)}k`
      : `${rounded}k`
  }

  const millions = num / 1_000_000
  const rounded = Math.round(millions * 10) / 10
  return rounded % 1 === 0
    ? `${Math.round(rounded)}M`
    : `${rounded}M`
}
