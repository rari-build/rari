use std::{rc::Rc, sync::Arc};

use ::deno_fetch::Options;
use deno_core::{Extension, ExtensionArguments, extension};
use deno_fetch::deno_fetch;
use deno_net::deno_net;
use deno_telemetry::deno_telemetry;
use deno_tls::deno_tls;
use deno_web::deno_web;

use super::{ExtensionTrait, lazy};

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

impl ExtensionTrait<WebOptions> for deno_fetch {
    const LAZY_INIT: bool = true;

    fn init(options: WebOptions) -> Extension {
        Self::init(fetch_options(&options))
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args(options: WebOptions) -> ExtensionArguments {
        Self::args(fetch_options(&options))
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

impl ExtensionTrait<WebOptions> for deno_net {
    const LAZY_INIT: bool = true;

    fn init(options: WebOptions) -> Extension {
        Self::init(
            options.root_cert_store_provider.clone(),
            options.unsafely_ignore_certificate_errors.clone(),
        )
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args(options: WebOptions) -> ExtensionArguments {
        Self::args(
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

impl ExtensionTrait<()> for deno_telemetry {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

extension!(
    init_web,
    deps = [init_utilities],
    esm_entry_point = "ext:init_web/init_web.ts",
    esm = [ dir "src/runtime/ext/web", "init_web.ts", "init_errors.ts", "shared_loaders.ts" ],
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

impl ExtensionTrait<WebOptions> for deno_web {
    const LAZY_INIT: bool = true;

    fn init(options: WebOptions) -> Extension {
        Self::init(options.blob_store, options.base_url, false, options.broadcast_channel)
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args(options: WebOptions) -> ExtensionArguments {
        Self::args(options.blob_store, options.base_url, false, options.broadcast_channel)
    }
}

impl ExtensionTrait<()> for deno_tls {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

fn fetch_options(options: &WebOptions) -> Options {
    Options {
        user_agent: options.user_agent.clone(),
        root_cert_store_provider: options.root_cert_store_provider.clone(),
        proxy: options.proxy.clone(),
        request_builder_hook: options.request_builder_hook,
        unsafely_ignore_certificate_errors: options.unsafely_ignore_certificate_errors.clone(),
        client_cert_chain_and_key: options.client_cert_chain_and_key.clone(),
        file_fetch_handler: Rc::clone(&options.file_fetch_handler),
        client_builder_hook: options.client_builder_hook,
        resolver: options.resolver.clone(),
    }
}

pub fn extensions(
    options: WebOptions,
    is_snapshot: bool,
) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    // init_fetch is built with is_snapshot=false even in the runtime path so its
    // esm entry point runs and installs cachedFetch via applyToGlobal.
    let fetch_is_snapshot = false;
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();

    lazy::register::<WebOptions, deno_web>(
        options.clone(),
        is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );
    lazy::register::<(), deno_telemetry>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<WebOptions, deno_net>(
        options.clone(),
        is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );
    lazy::register::<WebOptions, deno_fetch>(
        options.clone(),
        is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );
    lazy::register::<(), deno_tls>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<WebOptions, init_web>(
        options.clone(),
        is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );
    lazy::register::<(), init_telemetry>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<WebOptions, init_net>(
        options.clone(),
        is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );
    lazy::register::<WebOptions, init_fetch>(
        options,
        fetch_is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );

    (extensions, lazy_args)
}
