use super::node::resolvers::Resolver;
use super::web::PermissionsContainer;
use super::{ExtensionOptions, ExtensionTrait};
use ::deno_permissions::Permissions;
use deno_core::v8::{BackingStore, SharedRef};
use deno_core::{CrossIsolateStore, Extension, ExtensionFileSource, extension, op2};
use deno_runtime::permissions::RuntimePermissionDescriptorParser;
use deno_telemetry::OtelConfig;

use std::rc::Rc;
use std::sync::Arc;
use sys_traits::impls::RealSys;

#[op2]
#[string]
fn op_main_module() -> String {
    "".to_string()
}

#[op2(fast)]
#[smi]
fn op_ppid() -> u32 {
    1
}

fn build_permissions(
    _permissions_container: &PermissionsContainer,
) -> ::deno_permissions::PermissionsContainer {
    let parser = Arc::new(RuntimePermissionDescriptorParser::<RealSys>::new(RealSys));
    ::deno_permissions::PermissionsContainer::new(parser, Permissions::allow_all())
}

extension!(
    init_console,
    deps = [rari],
    esm_entry_point = "ext:init_console/init_console.js",
    esm = [ dir "src/runtime/ext/runtime", "init_console.js" ],
);
impl ExtensionTrait<()> for init_console {
    fn init((): ()) -> Extension {
        deno_terminal::colors::set_use_color(true);
        init_console::init()
    }
}

extension!(
    init_runtime,
    ops = [op_main_module, op_ppid],
    esm_entry_point = "ext:init_runtime/init_runtime.js",
    esm = [ dir "src/runtime/ext/runtime",  "init_runtime.js" ],
    state = |state| {
        let options = BootstrapOptions {
            args: vec![
                "--colors".to_string(),
            ],
            ..BootstrapOptions::default()
        };
        state.put(options);

        let container = state.borrow::<PermissionsContainer>();
        let permissions = build_permissions(container);
        state.put(permissions);
    },
    customizer = |e: &mut Extension| {
        use deno_core::ascii_str_include;
        e.esm_files.to_mut().push(
            ExtensionFileSource::new("ext:deno_features/flags.js", deno_features::JS_SOURCE)
        );
                        e.esm_files.to_mut().push(ExtensionFileSource::new(
            "ext:runtime_main/js/99_main.js",
            ascii_str_include!("./99_main.js")
        ));
        e.esm_entry_point = Some("ext:runtime_main/js/99_main.js");
    }
);
impl ExtensionTrait<()> for init_runtime {
    fn init((): ()) -> Extension {
        init_runtime::init()
    }
}

impl ExtensionTrait<()> for deno_runtime::runtime {
    fn init((): ()) -> Extension {
        let mut e = deno_runtime::runtime::init();
        e.esm_entry_point = None;
        e
    }
}

use deno_runtime::fmt_errors::format_js_error as deno_format_js_error;

fn format_js_error(error: &deno_core::error::JsError) -> String {
    deno_format_js_error(error, None)
}
use deno_runtime::ops::permissions::deno_permissions;
impl ExtensionTrait<()> for deno_permissions {
    fn init((): ()) -> Extension {
        deno_permissions::init()
    }
}

use deno_runtime::ops::worker_host::{CreateWebWorkerCb, deno_worker_host};
impl ExtensionTrait<(&ExtensionOptions, Option<CrossIsolateStore<SharedRef<BackingStore>>>)>
    for deno_worker_host
{
    fn init(
        options: (&ExtensionOptions, Option<CrossIsolateStore<SharedRef<BackingStore>>>),
    ) -> Extension {
        let options = WebWorkerCallbackOptions::new(options.0, options.1);
        let callback = create_web_worker_callback(options);
        deno_worker_host::init(callback, None)
    }
}

use deno_runtime::ops::web_worker::deno_web_worker;
impl ExtensionTrait<()> for deno_web_worker {
    fn init((): ()) -> Extension {
        deno_web_worker::init()
    }
}

use deno_process::deno_process;
impl ExtensionTrait<Arc<Resolver>> for deno_process {
    fn init(resolver: Arc<Resolver>) -> Extension {
        deno_process::init(Some(resolver))
    }
}

use deno_runtime::deno_os::{ExitCode, deno_os};
impl ExtensionTrait<()> for deno_os {
    fn init((): ()) -> Extension {
        deno_os::init(Some(ExitCode::default()))
    }
}

use deno_runtime::ops::bootstrap::deno_bootstrap;
impl ExtensionTrait<()> for deno_bootstrap {
    fn init((): ()) -> Extension {
        deno_bootstrap::init(None, false)
    }
}

use deno_runtime::ops::fs_events::deno_fs_events;
impl ExtensionTrait<()> for deno_fs_events {
    fn init((): ()) -> Extension {
        deno_fs_events::init()
    }
}

pub fn extensions(
    options: &ExtensionOptions,
    shared_array_buffer_store: Option<CrossIsolateStore<SharedRef<BackingStore>>>,
    is_snapshot: bool,
) -> Vec<Extension> {
    vec![
        deno_fs_events::build((), is_snapshot),
        deno_bootstrap::build((), is_snapshot),
        deno_os::build((), is_snapshot),
        deno_process::build(options.node_resolver.clone(), is_snapshot),
        deno_web_worker::build((), is_snapshot),
        deno_worker_host::build((options, shared_array_buffer_store), is_snapshot),
        deno_permissions::build((), is_snapshot),
        deno_runtime::runtime::build((), is_snapshot),
        init_console::build((), is_snapshot),
        init_runtime::build((), is_snapshot),
    ]
}

use deno_runtime::web_worker::{WebWorker, WebWorkerOptions, WebWorkerServiceOptions};
use deno_runtime::{BootstrapOptions, WorkerExecutionMode, WorkerLogLevel, colors};
#[derive(Clone)]
pub struct WebWorkerCallbackOptions {
    shared_array_buffer_store: Option<CrossIsolateStore<SharedRef<BackingStore>>>,
    node_resolver: Arc<Resolver>,
    root_cert_store_provider: Option<Arc<dyn deno_tls::RootCertStoreProvider>>,
    broadcast_channel: deno_web::InMemoryBroadcastChannel,
    unsafely_ignore_certificate_errors: Option<Vec<String>>,
    seed: Option<u64>,
    stdio: deno_io::Stdio,
    blob_store: Arc<deno_web::BlobStore>,
}
impl WebWorkerCallbackOptions {
    pub fn new(
        options: &ExtensionOptions,
        shared_array_buffer_store: Option<CrossIsolateStore<SharedRef<BackingStore>>>,
    ) -> Self {
        Self {
            shared_array_buffer_store,
            node_resolver: options.node_resolver.clone(),
            root_cert_store_provider: options.web.root_cert_store_provider.clone(),
            broadcast_channel: options.broadcast_channel.clone(),
            unsafely_ignore_certificate_errors: options
                .web
                .unsafely_ignore_certificate_errors
                .clone(),
            seed: options.crypto_seed,
            stdio: options.io_pipes.clone().unwrap_or_default(),
            blob_store: options.web.blob_store.clone(),
        }
    }
}

// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
fn create_web_worker_callback(options: WebWorkerCallbackOptions) -> Arc<CreateWebWorkerCb> {
    Arc::new(move |args| {
        let node_resolver = options.node_resolver.clone();
        let module_loader = Rc::new(crate::runtime::module_loader::RariModuleLoader::new());

        let create_web_worker_cb = create_web_worker_callback(options.clone());

        let mut feature_checker = deno_features::FeatureChecker::default();
        feature_checker.set_exit_cb(Box::new(|_, _| {}));

        let services = WebWorkerServiceOptions {
            root_cert_store_provider: options.root_cert_store_provider.clone(),
            module_loader,
            fs: node_resolver.filesystem(),
            node_services: Some(node_resolver.init_services()),
            blob_store: options.blob_store.clone(),
            broadcast_channel: options.broadcast_channel.clone(),
            shared_array_buffer_store: options.shared_array_buffer_store.clone(),
            compiled_wasm_module_store: None,
            main_inspector_session_tx: Default::default(),
            feature_checker: feature_checker.into(),
            npm_process_state_provider: Some(node_resolver.clone()),
            permissions: args.permissions,
            deno_rt_native_addon_loader: None,
            bundle_provider: None,
        };

        let options = WebWorkerOptions {
            name: args.name,
            main_module: args.main_module.clone(),
            worker_id: args.worker_id,
            bootstrap: BootstrapOptions {
                deno_version: env!("CARGO_PKG_VERSION").to_string(),
                args: vec![],
                cpu_count: std::thread::available_parallelism()
                    .map(std::num::NonZero::get)
                    .unwrap_or(1),
                log_level: WorkerLogLevel::default(),
                enable_op_summary_metrics: false,
                enable_testing_features: false,
                locale: deno_core::v8::icu::get_language_tag(),
                location: Some(args.main_module),
                color_level: colors::get_color_level(),
                unstable_features: vec![],
                user_agent: concat!("rari_", env!("CARGO_PKG_VERSION")).to_string(),
                inspect: false,
                has_node_modules_dir: node_resolver.has_node_modules_dir(),
                argv0: None,
                node_debug: None,
                node_ipc_init: None,
                mode: WorkerExecutionMode::Worker,
                serve_port: None,
                serve_host: None,
                otel_config: OtelConfig::default(),
                close_on_idle: false,
                no_legacy_abort: false,
                is_standalone: false,
                auto_serve: false,
            },
            extensions: vec![],
            startup_snapshot: None,
            unsafely_ignore_certificate_errors: options.unsafely_ignore_certificate_errors.clone(),
            seed: options.seed,
            create_web_worker_cb,
            format_js_error_fn: Some(Arc::new(format_js_error)),
            worker_type: args.worker_type,
            stdio: options.stdio.clone(),
            cache_storage_dir: None,
            trace_ops: None,
            close_on_idle: false,
            maybe_worker_metadata: None,
            maybe_coverage_dir: None,
            create_params: None,
            enable_stack_trace_arg_in_ops: false,
            enable_raw_imports: false,
        };
        WebWorker::bootstrap_from_options(services, options)
    })
}
