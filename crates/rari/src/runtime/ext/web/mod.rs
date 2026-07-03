use std::{rc::Rc, sync::Arc};

use deno_core::{Extension, extension};

use super::ExtensionTrait;

mod options;
mod permissions;

pub use options::WebOptions;
pub use permissions::{DefaultWebPermissions, PermissionsContainer, WebPermissions};

extension!(
    init_fetch,
    deps = [init_utilities],
    esm_entry_point = "ext:init_fetch/init_fetch.ts",
    esm = [ dir "src/runtime/ext/web", "init_fetch.ts" ],
);
impl ExtensionTrait<WebOptions> for init_fetch {
    #[expect(unused_variables)]
    fn init(options: WebOptions) -> Extension {
        Self::init()
    }
}
impl ExtensionTrait<WebOptions> for deno_fetch::deno_fetch {
    fn init(options: WebOptions) -> Extension {
        let options = deno_fetch::Options {
            user_agent: options.user_agent.clone(),
            root_cert_store_provider: options.root_cert_store_provider.clone(),
            proxy: options.proxy.clone(),
            request_builder_hook: options.request_builder_hook,
            unsafely_ignore_certificate_errors: options.unsafely_ignore_certificate_errors.clone(),
            client_cert_chain_and_key: options.client_cert_chain_and_key.clone(),
            file_fetch_handler: Rc::clone(&options.file_fetch_handler),
            client_builder_hook: options.client_builder_hook,
            resolver: options.resolver.clone(),
        };

        Self::init(options)
    }
}

extension!(
    init_net,
    deps = [init_utilities],
    esm_entry_point = "ext:init_net/init_net.ts",
    esm = [ dir "src/runtime/ext/web", "init_net.ts" ],
);
impl ExtensionTrait<WebOptions> for init_net {
    #[expect(unused_variables)]
    fn init(options: WebOptions) -> Extension {
        Self::init()
    }
}
impl ExtensionTrait<WebOptions> for deno_net::deno_net {
    fn init(options: WebOptions) -> Extension {
        Self::init(
            options.root_cert_store_provider.clone(),
            options.unsafely_ignore_certificate_errors.clone(),
        )
    }
}

extension!(
    init_telemetry,
    deps = [init_utilities],
    esm_entry_point = "ext:init_telemetry/init_telemetry.ts",
    esm = [ dir "src/runtime/ext/web", "init_telemetry.ts" ],
);
impl ExtensionTrait<()> for init_telemetry {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

impl ExtensionTrait<()> for deno_telemetry::deno_telemetry {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

extension!(
    init_web,
    deps = [init_utilities],
    esm_entry_point = "ext:init_web/init_web.ts",
    esm = [ dir "src/runtime/ext/web", "init_web.ts", "init_errors.ts" ],
    options = {
        permissions: Arc<dyn WebPermissions>
    },
    state = |state, config| state.put(PermissionsContainer(config.permissions)),
);
impl ExtensionTrait<WebOptions> for init_web {
    fn init(options: WebOptions) -> Extension {
        Self::init(options.permissions)
    }
}

impl ExtensionTrait<WebOptions> for deno_web::deno_web {
    fn init(options: WebOptions) -> Extension {
        Self::init(options.blob_store, options.base_url, false, options.broadcast_channel)
    }
}

impl ExtensionTrait<()> for deno_tls::deno_tls {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(options: WebOptions, is_snapshot: bool) -> Vec<Extension> {
    // init_fetch is built with is_snapshot=false even in the runtime
    // path so its esm entry point runs and installs cachedFetch via
    // applyToGlobal. The other init_* extensions keep is_snapshot as
    // passed (for_warmup strips their esm to avoid double-running
    // applyToGlobal side effects on top of V8-built-in globals).
    let fetch_is_snapshot = false;
    vec![
        deno_web::deno_web::build(options.clone(), is_snapshot),
        deno_telemetry::deno_telemetry::build((), is_snapshot),
        deno_net::deno_net::build(options.clone(), is_snapshot),
        deno_fetch::deno_fetch::build(options.clone(), is_snapshot),
        deno_tls::deno_tls::build((), is_snapshot),
        init_web::build(options.clone(), is_snapshot),
        init_telemetry::build((), is_snapshot),
        init_net::build(options.clone(), is_snapshot),
        init_fetch::build(options, fetch_is_snapshot),
    ]
}
