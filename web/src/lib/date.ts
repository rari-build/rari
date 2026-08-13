const DATE_ONLY_REGEX = /^(\d{4})-(\d{2})-(\d{2})$/

export function parseDate(dateString: string): Date {
  if (DATE_ONLY_REGEX.test(dateString)) return new Date(`${dateString}T12:00:00`)

  return new Date(dateString)
}

export function toDateOnly(dateString: string): string {
  const match = DATE_ONLY_REGEX.exec(dateString)
  if (match !== null) return dateString

  const isoPrefix = /^(\d{4}-\d{2}-\d{2})/.exec(dateString)
  if (isoPrefix !== null) return isoPrefix[1]

  const date = new Date(dateString)
  if (Number.isNaN(date.getTime())) return dateString

  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

export function formatBlogDate(dateString: string): string {
  if (!dateString) return ''
  return parseDate(dateString).toLocaleDateString('en-US', {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })
}

export function formatDate(dateString: string): string {
  if (!dateString) return ''
  return parseDate(dateString).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })
}
