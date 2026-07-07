export async function encodeFormData(formData: FormData): Promise<string> {
  let result = ''
  for (const [key, value] of formData) {
    result += `${key.length.toString(16)}:${key}`
    let stringValue: string
    if (typeof value === 'string') {
      stringValue = value
    }
    else {
      const arrayBuffer = await value.arrayBuffer()
      if (arrayBuffer.byteLength % 2 === 0) {
        stringValue = String.fromCodePoint(...new Uint16Array(arrayBuffer))
      }
      else {
        stringValue
          = String.fromCodePoint(
            ...new Uint16Array(arrayBuffer, 0, (arrayBuffer.byteLength - 1) / 2),
          )
          + String.fromCodePoint(
            new Uint8Array(arrayBuffer, arrayBuffer.byteLength - 1, 1)[0]!,
          )
      }
    }
    result += `${stringValue.length.toString(16)}:${stringValue}`
  }

  return result
}
