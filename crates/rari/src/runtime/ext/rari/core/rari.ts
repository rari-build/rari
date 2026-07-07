// @ts-nocheck
import { loadFullReactVendors, loadRscReactVendors } from 'ext:rari/react/vendor_loaders.ts'
import 'ext:rari/http/cookies.ts'
import 'ext:rari/http/headers.ts'
import 'ext:rari/cache/use_cache.ts'
import 'ext:rari/http/api_handler.ts'
import 'ext:rari/react/component_loader.ts'
import 'ext:rari/react/metadata_collector.ts'
import 'ext:rari/rsc/rsc_modules.ts'
import 'ext:rari/rsc/server_functions.ts'
import 'ext:rari/rsc/client_registry.ts'

if (!g['~rari'])
  g['~rari'] = {}

g['~rari'].loadFullReactVendors = loadFullReactVendors
g['~rari'].loadRscReactVendors = loadRscReactVendors
