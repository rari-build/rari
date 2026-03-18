const ObjectProperties = {
  nonEnumerable: { writable: true, enumerable: false, configurable: true },
  readOnly: { writable: false, enumerable: false, configurable: true },
  writeable: { writable: true, enumerable: true, configurable: true },
  getterOnly: { enumerable: true, configurable: true },

  apply: (value, type) => {
    return {
      value,
      ...ObjectProperties[type],
    }
  },
}
const nonEnumerable = value => ObjectProperties.apply(value, nonEnumerable)
const readOnly = value => ObjectProperties.apply(value, readOnly)
const writeable = value => ObjectProperties.apply(value, writeable)
function getterOnly(getter) {
  return {
    get: getter,
    set() {},
    ...ObjectProperties.getterOnly,
  }
}
const applyToGlobal = properties => Object.defineProperties(globalThis, properties)
const applyToDeno = properties => Object.defineProperties(globalThis.Deno, properties)

export {
  applyToDeno,
  applyToGlobal,
  getterOnly,
  nonEnumerable,
  readOnly,
  writeable,
}
