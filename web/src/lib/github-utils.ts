import process from 'node:process'

interface GitHubCommit {
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
    Accept: 'application/vnd.github.v3+json',
    ...(process.env.GITHUB_TOKEN && {
      Authorization: `Bearer ${process.env.GITHUB_TOKEN}`,
    }),
  }
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

    const commits: GitHubCommit[] = await response.json()

    if (commits.length === 0)
      return null

    return commits[0].commit.author.date
  }
  catch (error) {
    console.error(`Error fetching commit date for ${filePath}:`, error)
    return null
  }
}

export function formatDate(isoDate: string): string {
  const date = new Date(isoDate)
  return date.toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })
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

    const data: GitHubRepo = await response.json()
    return data
  }
  catch (error) {
    console.error('Error fetching repo info:', error)
    return null
  }
}

export async function getRepoStars(): Promise<number | null> {
  const repoInfo = await getRepoInfo()
  return repoInfo?.stargazers_count ?? null
}
