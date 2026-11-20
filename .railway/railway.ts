import type {
  ProjectDefinition,
  RailwayProvidedVariable,
  ReferencableServiceNode,
} from 'railway/iac'
import { defineRailway, github, project, service } from 'railway/iac'

export default defineRailway((): ProjectDefinition => {
  const web: ReferencableServiceNode<RailwayProvidedVariable> = service('web', {
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
  })

  return project('rari', { resources: [web] })
})
