interface NpmPackageInfo {
  'dist-tags': {
    latest: string
  }
}

async function fetchRariVersion(): Promise<string> {
  try {
    const response = await fetch('https://registry.npmjs.org/rari')
    if (!response.ok) {
      throw new Error(`Failed to fetch version: ${response.status}`)
    }

    const data: NpmPackageInfo = await response.json()
    return data['dist-tags'].latest
  }
  catch (error) {
    console.error('Error fetching rari version:', error)
    return '0.0.0'
  }
}

export default async function Version() {
  const version = await fetchRariVersion()
  return (
    <span>
      v
      {version}
    </span>
  )
}
