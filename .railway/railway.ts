import type {
  ProjectDefinition,
  RailwayProvidedVariable,
  ReferencableServiceNode,
} from 'railway/iac'
import { defineRailway, github, preserve, project, service } from 'railway/iac'

export default defineRailway((): ProjectDefinition => {
  const web: ReferencableServiceNode<RailwayProvidedVariable> = service('rari', {
    source: github('rari-build/rari', { branch: 'main' }),
    build: {
      builder: 'RAILPACK',
      buildCommand: 'pnpm --filter @rari/web build',
      watchPatterns: ['/web/**', '/pnpm-lock.yaml', '/pnpm-workspace.yaml'],
    },
    start: 'cd web && pnpm start',
    healthcheck: '/_rari/health',
    healthcheckTimeout: 300,
    deploy: {
      restartPolicyType: 'ON_FAILURE',
      restartPolicyMaxRetries: 3,
    },
    env: {
      GITHUB_TOKEN: preserve(),
      RAILPACK_CONFIG_FILE: preserve(),
      RAILPACK_DEPLOY_APT_PACKAGES: preserve(),
      VITE_POSTHOG_HOST: preserve(),
      VITE_POSTHOG_KEY: preserve(),
      VITE_SENTRY_DSN: preserve(),
    },
  })

  return project('rari.build', { resources: [web] })
})
