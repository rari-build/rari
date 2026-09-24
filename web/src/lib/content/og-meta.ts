import { readFile } from 'node:fs/promises'
import { extractBasicMetadata } from '@/lib/content/metadata'

export async function loadOgMeta(
  filePath: string,
  defaults: Readonly<{ title: string; description?: string }>,
): Promise<{ title: string; description?: string }> {
  try {
    const content = await readFile(filePath, 'utf-8')
    const metadata = extractBasicMetadata(content)

    return {
      title: metadata.title != null && metadata.title !== '' ? metadata.title : defaults.title,
      description:
        metadata.description != null && metadata.description !== ''
          ? metadata.description
          : defaults.description,
    }
  } catch {
    return { ...defaults }
  }
}
