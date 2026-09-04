import type { Buffer } from 'node:buffer'

export interface ImageDimensions {
  readonly width: number
  readonly height: number
}

function readUInt16BE(buffer: Buffer, offset: number): number {
  return buffer.readUInt16BE(offset)
}

function readUInt32BE(buffer: Buffer, offset: number): number {
  return buffer.readUInt32BE(offset)
}

function readUInt32LE(buffer: Buffer, offset: number): number {
  return buffer.readUInt32LE(offset)
}

function dimensionsFromPng(buffer: Buffer): ImageDimensions | null {
  if (buffer.length < 24) return null
  if (buffer.toString('ascii', 1, 4) !== 'PNG') return null
  return {
    width: readUInt32BE(buffer, 16),
    height: readUInt32BE(buffer, 20),
  }
}

function dimensionsFromGif(buffer: Buffer): ImageDimensions | null {
  if (buffer.length < 10) return null
  const header = buffer.toString('ascii', 0, 6)
  if (header !== 'GIF87a' && header !== 'GIF89a') return null
  return {
    width: buffer.readUInt16LE(6),
    height: buffer.readUInt16LE(8),
  }
}

function dimensionsFromJpeg(buffer: Buffer): ImageDimensions | null {
  if (buffer.length < 4 || buffer[0] !== 0xff || buffer[1] !== 0xd8) return null

  let offset = 2
  while (offset + 9 <= buffer.length) {
    if (buffer[offset] !== 0xff) {
      offset += 1
      continue
    }

    if (offset + 1 >= buffer.length) return null
    const marker = buffer[offset + 1]
    offset += 2

    const isSof =
      (marker >= 0xc0 && marker <= 0xc3) ||
      (marker >= 0xc5 && marker <= 0xc7) ||
      (marker >= 0xc9 && marker <= 0xcb) ||
      (marker >= 0xcd && marker <= 0xcf)

    if (isSof) {
      if (offset + 7 >= buffer.length) return null
      const size = readUInt16BE(buffer, offset)
      const components = buffer[offset + 7] ?? 0
      const required = 8 + 3 * components
      if (components < 1 || size < required) return null
      if (offset + size > buffer.length) return null
      const height = readUInt16BE(buffer, offset + 3)
      const width = readUInt16BE(buffer, offset + 5)
      if (width === 0 || height === 0) return null
      return { height, width }
    }

    if (marker === 0xd9 || marker === 0xda) return null

    if (offset + 1 >= buffer.length) return null
    const size = readUInt16BE(buffer, offset)
    if (size < 2) return null
    offset += size
  }

  return null
}

function readUint24LE(buffer: Buffer, offset: number): number {
  return (
    (buffer[offset] ?? 0) | ((buffer[offset + 1] ?? 0) << 8) | ((buffer[offset + 2] ?? 0) << 16)
  )
}

function dimensionsFromWebp(buffer: Buffer): ImageDimensions | null {
  if (buffer.length < 20) return null
  if (buffer.toString('ascii', 0, 4) !== 'RIFF') return null
  if (buffer.toString('ascii', 8, 12) !== 'WEBP') return null

  const chunk = buffer.toString('ascii', 12, 16)
  if (chunk === 'VP8X' && buffer.length >= 30) {
    return {
      width: readUint24LE(buffer, 24) + 1,
      height: readUint24LE(buffer, 27) + 1,
    }
  }

  if (chunk === 'VP8 ' && buffer.length >= 30) {
    return {
      width: buffer.readUInt16LE(26) & 0x3fff,
      height: buffer.readUInt16LE(28) & 0x3fff,
    }
  }

  if (chunk === 'VP8L' && buffer.length >= 25) {
    if (buffer[20] !== 0x2f) return null
    const bits = readUInt32LE(buffer, 21)
    return {
      width: (bits & 0x3fff) + 1,
      height: ((bits >> 14) & 0x3fff) + 1,
    }
  }

  return null
}

interface IsoBox {
  readonly type: string
  readonly headerSize: number
  readonly start: number
  readonly end: number
}

function readIsoBox(buffer: Buffer, offset: number, end: number): IsoBox | null {
  if (offset + 8 > end) return null

  let size = readUInt32BE(buffer, offset)
  const type = buffer.toString('ascii', offset + 4, offset + 8)
  let headerSize = 8

  if (size === 1) {
    if (offset + 16 > end) return null
    size = Number(buffer.readBigUInt64BE(offset + 8))
    headerSize = 16
  } else if (size === 0) {
    size = end - offset
  }

  const boxEnd = offset + size
  if (size < headerSize || boxEnd > end) return null

  return { type, headerSize, start: offset, end: boxEnd }
}

function* iterateIsoBoxes(buffer: Buffer, start: number, end: number): Generator<IsoBox> {
  let offset = start
  while (offset + 8 <= end) {
    const box = readIsoBox(buffer, offset, end)
    if (box == null) return
    yield box
    offset = box.end
  }
}

function readFullBoxVersionFlags(
  buffer: Buffer,
  box: IsoBox,
): {
  readonly version: number
  readonly flags: number
  readonly dataOffset: number
} | null {
  const versionOffset = box.start + box.headerSize
  if (versionOffset + 4 > box.end) return null
  const version = buffer[versionOffset] ?? 0
  const flags =
    ((buffer[versionOffset + 1] ?? 0) << 16) |
    ((buffer[versionOffset + 2] ?? 0) << 8) |
    (buffer[versionOffset + 3] ?? 0)
  return {
    version,
    flags,
    dataOffset: versionOffset + 4,
  }
}

function readIspeDimensions(buffer: Buffer, box: IsoBox): ImageDimensions | null {
  const full = readFullBoxVersionFlags(buffer, box)
  if (full == null) return null
  if (full.dataOffset + 8 > box.end) return null
  return {
    width: readUInt32BE(buffer, full.dataOffset),
    height: readUInt32BE(buffer, full.dataOffset + 4),
  }
}

function readPitmItemId(buffer: Buffer, box: IsoBox): number | null {
  const full = readFullBoxVersionFlags(buffer, box)
  if (full == null) return null
  if (full.version === 0) {
    if (full.dataOffset + 2 > box.end) return null
    return readUInt16BE(buffer, full.dataOffset)
  }
  if (full.dataOffset + 4 > box.end) return null
  return readUInt32BE(buffer, full.dataOffset)
}

function readIpmaAssociations(buffer: Buffer, box: IsoBox): Map<number, number[]> | null {
  const full = readFullBoxVersionFlags(buffer, box)
  if (full == null) return null
  if (full.dataOffset + 4 > box.end) return null

  const entryCount = readUInt32BE(buffer, full.dataOffset)
  let offset = full.dataOffset + 4
  const associations = new Map<number, number[]>()
  const largePropertyIndex = (full.flags & 1) !== 0

  for (let i = 0; i < entryCount; i += 1) {
    let itemId: number
    if (full.version < 1) {
      if (offset + 2 > box.end) return null
      itemId = readUInt16BE(buffer, offset)
      offset += 2
    } else {
      if (offset + 4 > box.end) return null
      itemId = readUInt32BE(buffer, offset)
      offset += 4
    }

    if (offset + 1 > box.end) return null
    const associationCount = buffer[offset] ?? 0
    offset += 1

    const propertyIndexes: number[] = []
    for (let j = 0; j < associationCount; j += 1) {
      if (largePropertyIndex) {
        if (offset + 2 > box.end) return null
        const value = readUInt16BE(buffer, offset)
        offset += 2
        propertyIndexes.push(value & 0x7fff)
      } else {
        if (offset + 1 > box.end) return null
        const value = buffer[offset] ?? 0
        offset += 1
        propertyIndexes.push(value & 0x7f)
      }
    }

    associations.set(itemId, propertyIndexes)
  }

  return associations
}

function findMetaBox(buffer: Buffer): IsoBox | null {
  for (const box of iterateIsoBoxes(buffer, 0, buffer.length)) {
    if (box.type === 'meta') return box
  }
  return null
}

function dimensionsFromAvif(buffer: Buffer): ImageDimensions | null {
  if (buffer.length < 12) return null
  if (buffer.toString('ascii', 4, 8) !== 'ftyp') return null

  const meta = findMetaBox(buffer)
  if (meta == null) return null
  const metaBody = meta.start + meta.headerSize + 4
  if (metaBody > meta.end) return null

  let primaryItemId: number | null = null
  const ispeByIndex = new Map<number, ImageDimensions>()
  let associations: Map<number, number[]> | null = null
  let firstIspe: ImageDimensions | null = null

  for (const box of iterateIsoBoxes(buffer, metaBody, meta.end)) {
    if (box.type === 'pitm') {
      primaryItemId = readPitmItemId(buffer, box)
      continue
    }

    if (box.type !== 'iprp') continue

    for (const child of iterateIsoBoxes(buffer, box.start + box.headerSize, box.end)) {
      if (child.type === 'ipco') {
        let propertyIndex = 1
        for (const property of iterateIsoBoxes(buffer, child.start + child.headerSize, child.end)) {
          if (property.type === 'ispe') {
            const dims = readIspeDimensions(buffer, property)
            if (dims != null) {
              ispeByIndex.set(propertyIndex, dims)
              firstIspe ??= dims
            }
          }
          propertyIndex += 1
        }
      } else if (child.type === 'ipma') {
        associations = readIpmaAssociations(buffer, child)
      }
    }
  }

  if (primaryItemId != null && associations != null) {
    const propertyIndexes = associations.get(primaryItemId)
    if (propertyIndexes != null) {
      for (const index of propertyIndexes) {
        const dims = ispeByIndex.get(index)
        if (dims != null) return dims
      }
    }
  }

  return firstIspe
}

export function readImageDimensions(buffer: Buffer): ImageDimensions | null {
  if (buffer.length < 4) return null

  if (buffer[0] === 0x89 && buffer.toString('ascii', 1, 4) === 'PNG')
    return dimensionsFromPng(buffer)

  if (buffer[0] === 0xff && buffer[1] === 0xd8) return dimensionsFromJpeg(buffer)

  if (buffer.toString('ascii', 0, 3) === 'GIF') return dimensionsFromGif(buffer)

  if (buffer.toString('ascii', 0, 4) === 'RIFF') return dimensionsFromWebp(buffer)

  return dimensionsFromAvif(buffer)
}
