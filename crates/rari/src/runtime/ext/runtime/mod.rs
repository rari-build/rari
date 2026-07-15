use std::{num::NonZero, rc::Rc, sync::Arc, thread};

use ::deno_permissions::{Permissions, PermissionsContainer as DenoPermissionsContainer};
use deno_core::{
    CrossIsolateStore, Extension, ExtensionArguments,
    error::JsError,
    extension,
    v8::{BackingStore, SharedRef, icu},
};
use deno_io::Stdio;
use deno_process::deno_process;
use deno_runtime::{
    BootstrapOptions, FeatureChecker, WorkerExecutionMode, WorkerLogLevel, colors,
    deno_inspector_server::MainInspectorSessionChannel,
    deno_os::{ExitCode, deno_os},
    fmt_errors::format_js_error as deno_format_js_error,
    ops::{
        bootstrap::deno_bootstrap,
        fs_events::deno_fs_events,
        permissions::deno_permissions,
        web_worker::deno_web_worker,
        worker_host::{CreateWebWorkerCb, deno_worker_host},
    },
    permissions::RuntimePermissionDescriptorParser,
    runtime,
    web_worker::{WebWorker, WebWorkerOptions, WebWorkerServiceOptions},
    worker::FormatJsErrorFn,
};
use deno_telemetry::OtelConfig;
use deno_tls::RootCertStoreProvider;
use deno_web::{BlobStore, InMemoryBroadcastChannel};
use sys_traits::impls::RealSys;

use super::{
    ExtensionOptions, ExtensionTrait, lazy, node::resolvers::Resolver,
    web::PermissionsContainer as WebPermissionsContainer,
};
use crate::runtime::module_loader::RariModuleLoader;

type WorkerHostOptions = (Arc<CreateWebWorkerCb>, Option<Arc<FormatJsErrorFn>>);

fn format_js_error(error: &JsError) -> String {
    deno_format_js_error(error, None)
}

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
        let mut ext = Self::init();

        ext.esm_files = ext
            .esm_files
            .iter()
            .filter(|file| {
                !file.specifier.contains("99_main.js")
                    && !file.specifier.contains("90_deno_ns.js")
                    && !file.specifier.contains("98_global_scope_shared.js")
                    && !file.specifier.contains("98_global_scope_worker.js")
                    && !file.specifier.contains("deno_features/flags.js")
            })
            .cloned()
            .collect::<Vec<_>>()
            .into();
        ext.esm_entry_point = None;

        ext
    }
}

impl ExtensionTrait<()> for deno_permissions {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

impl ExtensionTrait<WorkerHostOptions> for deno_worker_host {
    const LAZY_INIT: bool = true;

    fn init(options: WorkerHostOptions) -> Extension {
        Self::init(options.0, options.1)
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args(options: WorkerHostOptions) -> ExtensionArguments {
        Self::args(options.0, options.1)
    }
}

impl ExtensionTrait<()> for deno_web_worker {
    fn init((): ()) -> Extension {
        Self::init().disable()
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
        Self::init(None, false)
    }
}

impl ExtensionTrait<()> for deno_fs_events {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(
    options: &ExtensionOptions,
    shared_array_buffer_store: Option<CrossIsolateStore<SharedRef<BackingStore>>>,
    is_snapshot: bool,
) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let worker_host_options = (
        create_web_worker_callback(WebWorkerCallbackOptions::new(
            options,
            shared_array_buffer_store,
        )),
        Some(Arc::new(format_js_error) as Arc<FormatJsErrorFn>),
    );

    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();

    lazy::register::<(), deno_fs_events>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), deno_bootstrap>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), deno_os>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<Arc<Resolver>, deno_process>(
        Arc::clone(&options.node_resolver),
        is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );
    lazy::register::<(), deno_web_worker>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<WorkerHostOptions, deno_worker_host>(
        worker_host_options,
        is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );
    lazy::register::<(), deno_permissions>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), runtime>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), init_console>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), init_runtime>((), is_snapshot, &mut extensions, &mut lazy_args);

    (extensions, lazy_args)
}

#[derive(Clone)]
pub struct WebWorkerCallbackOptions {
    shared_array_buffer_store: Option<CrossIsolateStore<SharedRef<BackingStore>>>,
    node_resolver: Arc<Resolver>,
    root_cert_store_provider: Option<Arc<dyn RootCertStoreProvider>>,
    broadcast_channel: InMemoryBroadcastChannel,
    unsafely_ignore_certificate_errors: Option<Vec<String>>,
    seed: Option<u64>,
    stdio: Stdio,
    blob_store: Arc<BlobStore>,
}

impl WebWorkerCallbackOptions {
    pub fn new(
        options: &ExtensionOptions,
        shared_array_buffer_store: Option<CrossIsolateStore<SharedRef<BackingStore>>>,
    ) -> Self {
        Self {
            shared_array_buffer_store,
            node_resolver: Arc::clone(&options.node_resolver),
            root_cert_store_provider: options.web.root_cert_store_provider.clone(),
            broadcast_channel: options.broadcast_channel.clone(),
            unsafely_ignore_certificate_errors: options
                .web
                .unsafely_ignore_certificate_errors
                .clone(),
            seed: options.crypto_seed,
            stdio: options.io_pipes.clone().unwrap_or_default(),
            blob_store: Arc::clone(&options.web.blob_store),
        }
    }
}

fn create_web_worker_callback(options: WebWorkerCallbackOptions) -> Arc<CreateWebWorkerCb> {
    Arc::new(move |args| {
        let node_resolver = Arc::clone(&options.node_resolver);
        let module_loader = Rc::new(RariModuleLoader::new(Arc::clone(&node_resolver)));

        let create_web_worker_cb = create_web_worker_callback(options.clone());

        let mut feature_checker = FeatureChecker::default();
        feature_checker.set_exit_cb(Box::new(|_, _| {}));

        let services = WebWorkerServiceOptions {
            root_cert_store_provider: options.root_cert_store_provider.clone(),
            module_loader,
            fs: node_resolver.filesystem(),
            node_services: Some(node_resolver.init_services()),
            #[expect(
                clippy::clone_on_ref_ptr,
                reason = "Trait object coercion: Arc<BlobStore> -> Arc<dyn BlobStoreTrait>"
            )]
            blob_store: options.blob_store.clone(),
            broadcast_channel: options.broadcast_channel.clone(),
            shared_array_buffer_store: options.shared_array_buffer_store.clone(),
            compiled_wasm_module_store: None,
            main_inspector_session_tx: MainInspectorSessionChannel::default(),
            feature_checker: feature_checker.into(),
            #[expect(
                clippy::clone_on_ref_ptr,
                reason = "Trait object coercion: Arc<Resolver> -> Arc<dyn NpmProcessStateProvider>"
            )]
            npm_process_state_provider: Some(node_resolver.clone()),
            permissions: args.permissions,
            deno_rt_native_addon_loader: None,
            bundle_provider: None,
        };

        let options = WebWorkerOptions {
            name: args.name,
            main_module: args.main_module.clone(),
            worker_id: args.worker_id,
            maybe_cpu_prof_config: None,
            bootstrap: BootstrapOptions {
                deno_version: env!("CARGO_PKG_VERSION").to_string(),
                args: vec![],
                cpu_count: thread::available_parallelism().map(NonZero::get).unwrap_or(1),
                log_level: WorkerLogLevel::default(),
                enable_testing_features: false,
                locale: icu::get_language_tag(),
                location: Some(args.main_module),
                color_level: colors::get_color_level(),
                unstable_features: vec![],
                user_agent: concat!("rari_", env!("CARGO_PKG_VERSION")).to_string(),
                inspect: false,
                has_node_modules_dir: node_resolver.has_node_modules_dir(),
                argv0: None,
                node_debug: None,
                node_cluster_unique_id: None,
                node_cluster_sched_policy: None,
                node_ipc_init: None,
                mode: WorkerExecutionMode::Worker,
                serve_port: None,
                serve_host: None,
                otel_config: OtelConfig::default(),
                close_on_idle: false,
                no_legacy_abort: false,
                is_standalone: false,
                auto_serve: false,
                disable_offscreen_canvas: false,
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
            maybe_worker_metadata: args.maybe_worker_metadata,
            maybe_main_module_blob: args.maybe_main_module_blob,
            maybe_coverage_dir: None,
            create_params: None,
            enable_stack_trace_arg_in_ops: false,
            enable_raw_imports: false,
            wait_for_debugger_on_start: args.wait_for_debugger_on_start,
            wait_for_page_wait_for_debugger: args.wait_for_page_wait_for_debugger,
            residual_lazy_js_sources: &[],
            residual_lazy_esm_sources: &[],
        };
        WebWorker::bootstrap_from_options(services, options)
    })
}
