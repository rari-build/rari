import type { Plugin } from 'rolldown-vite'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react-oxc'
import { rari, rariRouter } from 'rari'
import { defineConfig } from 'rolldown-vite'

function versionPlugin(): Plugin {
  return {
    name: 'version-plugin',
    config() {
      try {
        const rariPackageJson = JSON.parse(
          readFileSync(join(process.cwd(), '../packages/rari/package.json'), 'utf-8'),
        )
        return {
          define: {
            __RARI_VERSION__: JSON.stringify(rariPackageJson.version),
          },
        }
      }
      catch {
        console.warn('Could not read rari version, using fallback')
        return {
          define: {
            __RARI_VERSION__: JSON.stringify('0.0.0'),
          },
        }
      }
    },
  }
}

export default defineConfig({
  plugins: [rari(), rariRouter(), react(), tailwindcss(), versionPlugin()],
})
