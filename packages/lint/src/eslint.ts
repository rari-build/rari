import type { Linter } from 'eslint'
import gitignore from 'eslint-config-flat-gitignore'
import { ignorePatterns } from './ignores'
import { pnpmConfigs } from './pnpm'
import { jsonSortConfigs } from './sort'

const configs: Linter.Config[] = [
  gitignore(),
  {
    ignores: [...ignorePatterns],
  },
  ...pnpmConfigs,
  ...jsonSortConfigs,
]

export default configs
