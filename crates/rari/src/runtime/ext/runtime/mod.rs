use std::sync::Arc;

use ::deno_os::{ExitCode, deno_os};
use ::deno_permissions::{
    Permissions, PermissionsContainer as DenoPermissionsContainer,
    RuntimePermissionDescriptorParser,
};
use deno_core::{Extension, ExtensionArguments, extension};
use deno_process::deno_process;
use rari_runtime::{
    BootstrapOptions, colors,
    ops::{
        bootstrap::deno_bootstrap, permissions::deno_permissions, worker_host::deno_worker_host,
    },
    runtime,
};
use sys_traits::impls::RealSys;

use super::{
    ExtensionOptions, ExtensionTrait, lazy, node::resolvers::Resolver,
    web::PermissionsContainer as WebPermissionsContainer,
};

fn build_permissions(permissions_container: &WebPermissionsContainer) -> DenoPermissionsContainer {
    let parser = Arc::new(RuntimePermissionDescriptorParser::<RealSys>::new(RealSys));
    let permissions =
        permissions_container.0.to_deno_permissions(parser.as_ref()).unwrap_or_else(|err| {
            tracing::warn!("Failed to derive Deno permissions from web permissions: {err}");
            Permissions::none_without_prompt()
        });
    DenoPermissionsContainer::new(parser, permissions)
}

extension!(
    init_console,
    deps = [init_utilities],
    esm_entry_point = "ext:init_console/init_console.ts",
    esm = [ dir "src/runtime/ext/runtime", "init_console.ts" ],
    lazy_loaded_esm = [
        dir "src/runtime/ext/runtime",
        "ext:runtime/98_global_scope_shared.js" = "node_console_scope.ts",
    ],
);

extension!(
    init_runtime,
    esm_entry_point = "ext:init_runtime/init_runtime.ts",
    esm = [ dir "src/runtime/ext/runtime",  "init_runtime.ts" ],
    lazy_loaded_esm = [ dir "src/runtime/ext/runtime", "init_node_bootstrap.ts" ],
    state = |state| {
        let options = BootstrapOptions {
            args: vec![
                "--colors".to_string(),
            ],
            ..BootstrapOptions::default()
        };
        state.put(options);

        let container = state.borrow::<WebPermissionsContainer>();
        let permissions = build_permissions(container);
        state.put(permissions);
    },
);

impl ExtensionTrait<()> for init_console {
    fn init((): ()) -> Extension {
        colors::set_use_color(true);
        Self::init()
    }
}

impl ExtensionTrait<()> for init_runtime {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

impl ExtensionTrait<()> for runtime {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

impl ExtensionTrait<()> for deno_permissions {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

impl ExtensionTrait<()> for deno_worker_host {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

impl ExtensionTrait<Arc<Resolver>> for deno_process {
    const LAZY_INIT: bool = true;

    fn init(resolver: Arc<Resolver>) -> Extension {
        Self::init(Some(resolver))
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args(resolver: Arc<Resolver>) -> ExtensionArguments {
        Self::args(Some(resolver))
    }
}

impl ExtensionTrait<()> for deno_os {
    const LAZY_INIT: bool = true;

    fn init((): ()) -> Extension {
        Self::init(Some(ExitCode::default()))
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args((): ()) -> ExtensionArguments {
        Self::args(Some(ExitCode::default()))
    }
}

impl ExtensionTrait<()> for deno_bootstrap {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(
    options: &ExtensionOptions,
    is_snapshot: bool,
) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();

    lazy::register::<(), deno_bootstrap>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), deno_os>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<Arc<Resolver>, deno_process>(
        Arc::clone(&options.node_resolver),
        is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );
    lazy::register::<(), deno_worker_host>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), deno_permissions>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), runtime>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), init_console>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), init_runtime>((), is_snapshot, &mut extensions, &mut lazy_args);

    (extensions, lazy_args)
}
