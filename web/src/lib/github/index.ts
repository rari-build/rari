import process from 'node:process'

const GITHUB_REPO = 'rari-build/rari'
const GITHUB_API_BASE = 'https://api.github.com'

export function getGitHubEditUrl(repoPath: string): string {
  const encodedPath = repoPath
    .split('/')
    .map(segment => encodeURIComponent(segment))
    .join('/')
  return `https://github.com/${GITHUB_REPO}/edit/main/${encodedPath}`
}

function githubHeaders(): HeadersInit {
  const headers: Record<string, string> = {
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'rari-build/rari',
  }
  const token = process.env.GITHUB_TOKEN
  if (token != null && token !== '') headers.Authorization = `Bearer ${token}`
  return headers
}

async function githubJson(url: string): Promise<unknown> {
  try {
    const response = await fetch(url, {
      headers: githubHeaders(),
      rari: { revalidate: 3600 },
    })
    if (!response.ok) {
      console.warn(`GitHub ${response.status}: ${url}`)
      return null
    }
    return await response.json()
  } catch (error) {
    console.error(`GitHub fetch failed: ${url}`, error)
    return null
  }
}

function read(obj: unknown, key: string): unknown {
  if (typeof obj !== 'object' || obj === null || !(key in obj)) return undefined
  return Reflect.get(obj, key)
}

function commitDate(data: unknown): string | null {
  if (!Array.isArray(data) || data.length === 0) return null
  const date = read(read(read(data[0], 'commit'), 'author'), 'date')
  return typeof date === 'string' ? date : null
}

function commitSha(data: unknown): string | null {
  if (!Array.isArray(data) || data.length === 0) return null
  const sha = read(data[0], 'sha')
  return typeof sha === 'string' ? sha.slice(0, 8) : null
}

export async function getLastCommitDate(filePath: string): Promise<string | null> {
  return commitDate(
    await githubJson(
      `${GITHUB_API_BASE}/repos/${GITHUB_REPO}/commits?path=${encodeURIComponent(filePath)}&page=1&per_page=1`,
    ),
  )
}

export async function getRepoStars(): Promise<number | null> {
  const stars = read(
    await githubJson(`${GITHUB_API_BASE}/repos/${GITHUB_REPO}`),
    'stargazers_count',
  )
  return typeof stars === 'number' ? stars : null
}

export async function getLatestCommitHash(): Promise<string | null> {
  return commitSha(
    await githubJson(`${GITHUB_API_BASE}/repos/${GITHUB_REPO}/commits?path=web&page=1&per_page=1`),
  )
}

export async function getLatestRariVersion(): Promise<string> {
  const data = await githubJson(`${GITHUB_API_BASE}/repos/${GITHUB_REPO}/releases?per_page=30`)
  if (!Array.isArray(data)) return '0.0.0'
  for (const release of data) {
    const tag = read(release, 'tag_name')
    if (typeof tag === 'string' && tag.startsWith('rari@') && tag.length > 5) {
      return tag.slice('rari@'.length)
    }
  }
  return '0.0.0'
}
