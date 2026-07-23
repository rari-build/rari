/// <reference path="../core/types.d.ts" />

interface ResolveResult {
  success: boolean
  registered: number
  component?: string
  functions: string[]
}

;(function initializeServerFunctions() {
  g['~rari'] ??= {}
  g['~rari'].registeredServerFunctions ??= new Set()

  g.resolveServerFunctionsForComponent = async function resolveServerFunctionsForComponent(
    componentId?: string,
  ): Promise<ResolveResult> {
    const currentComponent = componentId ?? g['~render']?.currentComponent

    const manifest = g['~rari']!.serverManifest ?? {}
    const functionNames = Object.keys(manifest).filter(
      key => key.includes('#') || key.includes(':'),
    )

    const registered = g['~rari']!.registeredServerFunctions!
    const newlyRegistered: string[] = []

    for (const functionName of functionNames) {
      if (functionName.startsWith('~rari_')) continue

      const entry = manifest[functionName]!
      const exportName = entry.name ?? functionName.split(/[#:]/).pop()
      if (exportName == null || exportName === '' || exportName === 'default') continue

      if (registered.has(functionName)) continue

      registered.add(functionName)
      newlyRegistered.push(functionName)
    }

    return Promise.resolve({
      success: true,
      registered: newlyRegistered.length,
      component: currentComponent,
      functions: newlyRegistered,
    })
  }

  g.executeServerFunction = async function executeServerFunction(
    functionName: string,
    args: readonly unknown[] = [],
  ): Promise<unknown> {
    let serverFunction: ((...args: readonly unknown[]) => unknown) | null = null

    if (g.RscModuleManager?.getFunction)
      serverFunction = g.RscModuleManager.getFunction(functionName)
    else if (g.getServerFunction) serverFunction = g.getServerFunction(functionName)

    if (!serverFunction) throw new Error(`Server function '${functionName}' not found`)

    const result = await serverFunction(...args)

    return result
  }

  g.createEnhancedServerFunctionPromise = async function createEnhancedServerFunctionPromise(
    functionName: string,
    args: readonly unknown[] = [],
  ): Promise<unknown> {
    if (g.RscModuleManager?.createPromise)
      return g.RscModuleManager.createPromise(functionName, args)

    return g.executeServerFunction!(functionName, args)
  }

  g.isServerFunctionRegistered = function isServerFunctionRegistered(
    functionName: string,
  ): boolean {
    return g['~rari']!.registeredServerFunctions?.has(functionName) ?? false
  }

  g.clearServerFunctionCache = function clearServerFunctionCache(): void {
    g['~rari']!.registeredServerFunctions!.clear()
  }

  g.ServerFunctions = {
    resolve: g.resolveServerFunctionsForComponent,
    execute: g.executeServerFunction,
    createPromise: g.createEnhancedServerFunctionPromise,
    isRegistered: g.isServerFunctionRegistered,
    clear: g.clearServerFunctionCache,
  }

  return {
    initialized: true,
    timestamp: Date.now(),
    extension: 'server_functions',
    registeredCount: g['~rari'].registeredServerFunctions.size,
  }
})()
