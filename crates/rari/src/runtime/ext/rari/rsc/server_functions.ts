/// <reference path="../core/types.d.ts" />

interface ResolveResult {
  success: boolean
  registered: number
  component?: string
  functions: string[]
}

interface ServerFunctionOptions {
  componentId?: string
}

(function initializeServerFunctions() {
  if (!g['~serverFunctions'])
    g['~serverFunctions'] = {}

  if (!g['~serverFunctions'].registered)
    g['~serverFunctions'].registered = new Set()

  if (!g['~serverFunctions'].exported)
    g['~serverFunctions'].exported = {}

  if (!g['~serverFunctions'].all)
    g['~serverFunctions'].all = {}

  g.resolveServerFunctionsForComponent = async function resolveServerFunctionsForComponent(
    componentId?: string,
  ): Promise<ResolveResult> {
    const currentComponent
      = componentId || g['~render']?.currentComponent

    const serverFunctions = g['~serverFunctions']!.exported || {}
    const functionNames = Object.keys(serverFunctions)

    let registeredCount = 0

    for (const functionName of functionNames) {
      const serverFunction = serverFunctions[functionName]
      if (typeof serverFunction === 'function') {
        if (functionName.startsWith('~rari_') || functionName === 'default')
          continue

        g['~serverFunctions']!.registered!.add(functionName)
        registeredCount++
      }
    }

    return {
      success: true,
      registered: registeredCount,
      component: currentComponent,
      functions: [...g['~serverFunctions']!.registered!],
    }
  }

  g.executeServerFunction = async function executeServerFunction(
    functionName: string,
    args: unknown[] = [],
  ): Promise<unknown> {
    let serverFunction: ((...args: any[]) => any) | null = null

    if (g.RscModuleManager?.getFunction)
      serverFunction = g.RscModuleManager.getFunction(functionName)
    else if (g.getServerFunction)
      serverFunction = g.getServerFunction(functionName)

    if (!serverFunction)
      throw new Error(`Server function '${functionName}' not found`)

    const result = await serverFunction(...args)

    return result
  }

  g.createEnhancedServerFunctionPromise = function createEnhancedServerFunctionPromise(
    functionName: string,
    args: unknown[] = [],
    options: ServerFunctionOptions = {},
  ): Promise<unknown> {
    const { componentId } = options

    if (g.RscModuleManager?.createPromise) {
      const promise = g.RscModuleManager.createPromise(
        functionName,
        args,
      )

      if (componentId)
        (promise as any)['~rsc_component_id'] = componentId

      return promise
    }

    return g.executeServerFunction!(functionName, args)
  }

  g.isServerFunctionRegistered = function isServerFunctionRegistered(
    functionName: string,
  ): boolean {
    return g['~serverFunctions']!.registered?.has(functionName) || false
  }

  g.clearServerFunctionCache = function clearServerFunctionCache(): void {
    g['~serverFunctions']!.registered!.clear()
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
    registeredCount: g['~serverFunctions'].registered?.size || 0,
  }
})()
