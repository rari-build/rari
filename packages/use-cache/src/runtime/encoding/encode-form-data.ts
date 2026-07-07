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
      const bytes = new Uint8Array(arrayBuffer)
      let binary = ''
      for (const byte of bytes)
        binary += String.fromCharCode(byte)
      stringValue = `base64:${btoa(binary)}`
    }
    result += `${stringValue.length.toString(16)}:${stringValue}`
  }

  return result
}
