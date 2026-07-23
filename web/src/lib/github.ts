import process from 'node:process'

interface GitHubCommit {
  sha: string
  commit: {
    author: {
      date: string
    }
  }
}

interface GitHubRepo {
  stargazers_count: number
  forks_count: number
  watchers_count: number
  open_issues_count: number
  updated_at: string
}

const GITHUB_REPO = 'rari-build/rari'
const GITHUB_API_BASE = 'https://api.github.com'

function getGitHubHeaders(): HeadersInit {
  return {
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'rari-build/rari',
    ...(process.env.GITHUB_TOKEN != null &&
      process.env.GITHUB_TOKEN !== '' && {
        Authorization: `Bearer ${process.env.GITHUB_TOKEN}`,
      }),
  }
}

function isGitHubCommit(value: unknown): value is GitHubCommit {
  if (typeof value !== 'object' || value === null) return false

  const sha: unknown = Reflect.get(value, 'sha')
  if (typeof sha !== 'string') return false

  const commit: unknown = Reflect.get(value, 'commit')
  if (typeof commit !== 'object' || commit === null) return false

  const author: unknown = Reflect.get(commit, 'author')
  if (typeof author !== 'object' || author === null) return false

  return typeof Reflect.get(author, 'date') === 'string'
}

function isGitHubCommitArray(value: unknown): value is GitHubCommit[] {
  return Array.isArray(value) && value.every(isGitHubCommit)
}

function isGitHubRepo(value: unknown): value is GitHubRepo {
  if (typeof value !== 'object' || value === null) return false

  return (
    typeof Reflect.get(value, 'stargazers_count') === 'number' &&
    typeof Reflect.get(value, 'forks_count') === 'number' &&
    typeof Reflect.get(value, 'watchers_count') === 'number' &&
    typeof Reflect.get(value, 'open_issues_count') === 'number' &&
    typeof Reflect.get(value, 'updated_at') === 'string'
  )
}

export async function getLastCommitDate(filePath: string): Promise<string | null> {
  try {
    const url = `${GITHUB_API_BASE}/repos/${GITHUB_REPO}/commits?path=${encodeURIComponent(filePath)}&page=1&per_page=1`

    const response = await fetch(url, {
      headers: getGitHubHeaders(),
      rari: { revalidate: 3600 },
    })

    if (!response.ok) {
      console.warn(`Failed to fetch commit date for ${filePath}: ${response.status}`)
      return null
    }

    const commits: unknown = await response.json()

    if (!isGitHubCommitArray(commits) || commits.length === 0) return null

    return commits[0].commit.author.date
  } catch (error) {
    console.error(`Error fetching commit date for ${filePath}:`, error)
    return null
  }
}

async function getRepoInfo(): Promise<GitHubRepo | null> {
  try {
    const url = `${GITHUB_API_BASE}/repos/${GITHUB_REPO}`

    const response = await fetch(url, {
      headers: getGitHubHeaders(),
      rari: { revalidate: 3600 },
    })

    if (!response.ok) {
      console.warn(`Failed to fetch repo info: ${response.status}`)
      return null
    }

    const data: unknown = await response.json()
    if (!isGitHubRepo(data)) return null

    return data
  } catch (error) {
    console.error('Error fetching repo info:', error)
    return null
  }
}

export async function getRepoStars(): Promise<number | null> {
  const repoInfo = await getRepoInfo()
  return repoInfo?.stargazers_count ?? null
}
export async function getLatestCommitHash(): Promise<string | null> {
  try {
    const url = `${GITHUB_API_BASE}/repos/${GITHUB_REPO}/commits?path=web&page=1&per_page=1`

    const response = await fetch(url, {
      headers: getGitHubHeaders(),
      rari: { revalidate: 3600 },
    })

    if (!response.ok) {
      console.warn(`Failed to fetch latest commit: ${response.status}`)
      return null
    }

    const commits: unknown = await response.json()

    if (!isGitHubCommitArray(commits) || commits.length === 0) return null

    return commits[0].sha.substring(0, 8)
  } catch (error) {
    console.error('Error fetching latest commit:', error)
    return null
  }
}
