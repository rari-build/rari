const hasOwn = Object.prototype.hasOwnProperty

function safeHasOwnProperty(obj, key) {
  if (obj == null || typeof obj !== 'object')
    return false
  return hasOwn.call(obj, key)
}

function safeGetProperty(obj, key) {
  if (obj == null || typeof obj !== 'object')
    return undefined
  if (safeHasOwnProperty(obj, key))
    return obj[key]
  return undefined
}

function isDangerousProperty(key) {
  return (
    key === '__proto__'
    || key === 'constructor'
    || key === 'prototype'
    || key === '__defineGetter__'
    || key === '__defineSetter__'
    || key === '__lookupGetter__'
    || key === '__lookupSetter__'
  )
}

function sanitizeValue(value) {
  if (value == null)
    return value

  if (Array.isArray(value))
    return value.map(item => sanitizeValue(item))

  if (typeof value === 'object') {
    const sanitized = {}
    for (const key in value) {
      if (safeHasOwnProperty(value, key) && !isDangerousProperty(key))
        sanitized[key] = sanitizeValue(value[key])
    }
    return sanitized
  }

  return value
}

if (typeof globalThis !== 'undefined') {
  if (!globalThis['~rari'])
    globalThis['~rari'] = {}
  globalThis['~rari'].safePropertyAccess = {
    safeHasOwnProperty,
    safeGetProperty,
    isDangerousProperty,
    sanitizeValue,
  }
}
