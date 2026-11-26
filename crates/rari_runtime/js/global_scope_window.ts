// Copyright 2018-2026 the Deno authors. MIT license.

import { core, primordials } from 'ext:core/mod.js'
import { op_bootstrap_language, op_bootstrap_numcpus, op_bootstrap_user_agent } from 'ext:core/ops'
import { NavigatorUAData, navigatorUAData } from 'ext:runtime/navigator_user_agent_data.ts'
import { alert, confirm, prompt } from 'ext:runtime/prompt.ts'

const promptModule = { alert, confirm, prompt }

const {
  ObjectDefineProperties,
  ObjectPrototypeIsPrototypeOf,
  SymbolFor,
  StringPrototypeToUpperCase,
  StringPrototypeSlice,
} = primordials

const location = core.loadExtScript('ext:deno_web/12_location.js')
const console = core.loadExtScript('ext:deno_web/01_console.js')
const webidl = core.loadExtScript('ext:deno_webidl/00_webidl.js')
const globalInterfaces = core.loadExtScript('ext:deno_web/04_global_interfaces.js')
const loadLocks = core.createLazyLoader<LocksModule>('ext:deno_web/locks.js')
// `alert`/`confirm`/`prompt` are rarely accessed on the cold-start path. Their
// implementations come from `lazy_loaded_js` polyfills that we don't want
// to evaluate at snapshot build time. The descriptor entries below resolve
// each polyfill on first read.

function getNavigatorPlatform(arch: string, platform: string): string {
  switch (platform) {
    case 'darwin':
      // On macOS, modern browsers return 'MacIntel' even if running on Apple Silicon.
      return 'MacIntel'

    case 'windows':
      // On Windows, modern browsers return 'Win32' even if running on a 64-bit version of Windows.
      // https://developer.mozilla.org/en-US/docs/Web/API/Navigator/platform#usage_notes
      return 'Win32'

    case 'linux':
      return `Linux ${arch}`

    case 'freebsd':
      if (arch === 'x86_64') {
        return 'FreeBSD amd64'
      }
      return `FreeBSD ${arch}`

    case 'solaris':
      return `SunOS ${arch}`

    case 'aix':
      return 'AIX'

    default: {
      const platformName = platform.length === 0 ? '' : platform
      return `${StringPrototypeToUpperCase(platformName)}${StringPrototypeSlice(
        platform,
        1,
      )} ${arch}`
    }
  }
}

let NavigatorPrototype: object

class Navigator {
  constructor() {
    webidl.illegalConstructor()
  }

  [SymbolFor('Deno.privateCustomInspect')](
    inspect: (value: unknown, options: unknown) => string,
    inspectOptions: unknown,
  ) {
    return inspect(
      console.createFilteredInspectProxy({
        object: this,
        evaluate: ObjectPrototypeIsPrototypeOf(NavigatorPrototype, this),
        keys: [
          'hardwareConcurrency',
          'userAgent',
          'language',
          'languages',
          'platform',
          'userAgentData',
        ],
      }),
      inspectOptions,
    )
  }
}

const navigator = webidl.createBranded(Navigator)

function memoizeLazy<T>(f: () => T): () => T {
  let v_: T | null = null
  return () => {
    v_ ??= f()
    return v_
  }
}

const numCpus = memoizeLazy(() => op_bootstrap_numcpus())
const userAgent = memoizeLazy(() => op_bootstrap_user_agent())
const language = memoizeLazy(() => op_bootstrap_language())
const platform = memoizeLazy(() => getNavigatorPlatform(core.build.arch, core.build.os))

ObjectDefineProperties(Navigator.prototype, {
  hardwareConcurrency: {
    __proto__: null,
    configurable: true,
    enumerable: true,
    get() {
      webidl.assertBranded(this, NavigatorPrototype)
      return numCpus()
    },
  },
  userAgent: {
    __proto__: null,
    configurable: true,
    enumerable: true,
    get() {
      webidl.assertBranded(this, NavigatorPrototype)
      return userAgent()
    },
  },
  language: {
    __proto__: null,
    configurable: true,
    enumerable: true,
    get() {
      webidl.assertBranded(this, NavigatorPrototype)
      return language()
    },
  },
  languages: {
    __proto__: null,
    configurable: true,
    enumerable: true,
    get() {
      webidl.assertBranded(this, NavigatorPrototype)
      return [language()]
    },
  },
  locks: {
    __proto__: null,
    configurable: true,
    enumerable: true,
    get() {
      webidl.assertBranded(this, NavigatorPrototype)
      return loadLocks().locks
    },
  },
  platform: {
    __proto__: null,
    configurable: true,
    enumerable: true,
    get() {
      webidl.assertBranded(this, NavigatorPrototype)
      return platform()
    },
  },
  userAgentData: {
    __proto__: null,
    configurable: true,
    enumerable: true,
    get() {
      webidl.assertBranded(this, NavigatorPrototype)
      return navigatorUAData
    },
  },
})
NavigatorPrototype = Navigator.prototype

const mainRuntimeGlobalProperties = {
  Location: location.locationConstructorDescriptor,
  location: location.locationDescriptor,
  Window: globalInterfaces.windowConstructorDescriptor,
  self: core.propGetterOnly(() => globalThis),
  Navigator: core.propNonEnumerable(Navigator),
  navigator: core.propGetterOnly(() => navigator),
  NavigatorUAData: core.propNonEnumerable(NavigatorUAData),
  alert: core.propWritableLazyLoaded(
    p => p.alert,
    () => promptModule,
  ),
  confirm: core.propWritableLazyLoaded(
    p => p.confirm,
    () => promptModule,
  ),
  prompt: core.propWritableLazyLoaded(
    p => p.prompt,
    () => promptModule,
  ),
}

export { mainRuntimeGlobalProperties, memoizeLazy }
