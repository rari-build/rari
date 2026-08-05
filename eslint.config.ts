import rariLint from '@rari/lint/eslint'
import { monorepoEslintConfigs } from './.config/lint/monorepo'

export default [...rariLint, ...monorepoEslintConfigs]
