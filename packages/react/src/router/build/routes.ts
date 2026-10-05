// The file-system route scanner is framework-agnostic and lives in @rari/core;
// React uses its default (Next-style) conventions.
export { generateAppRouteManifest, isGroupSegment } from '@rari/core/router'
export type { AppRouteGeneratorOptions, RouteConventions } from '@rari/core/router'
