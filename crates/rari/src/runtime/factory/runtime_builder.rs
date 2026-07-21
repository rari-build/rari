use std::{
    borrow::Cow,
    rc::Rc,
    sync::{Arc, LazyLock},
};

use cow_utils::CowUtils;
use deno_core::{Extension, JsRuntime, RuntimeOptions};
use deno_runtime::BootstrapOptions;
use rari_error::RariError;
use rustc_hash::FxHashMap;

use crate::runtime::{
    ext,
    factory::{
        create_params::runtime_create_params,
        utils::constants::{ENV_INJECTION_SCRIPT, MODULE_CHECK_SCRIPT, NODE_BOOTSTRAP_SCRIPT},
    },
    module_loader::RariModuleLoader,
    ops::{self, StreamOpState},
};

const NODE_CONSOLE_SCOPE_SPECIFIER: &str = "ext:runtime/98_global_scope_shared.js";
const NODE_CONSOLE_SCOPE_SOURCE: &str = include_str!("../ext/runtime/node_console_scope.ts");

fn residual_lazy_esm_sources() -> &'static [(&'static str, &'static str)] {
    static SOURCES: LazyLock<Vec<(&'static str, &'static str)>> = LazyLock::new(|| {
        let mut sources = RESIDUAL_LAZY_ESM_SOURCES.to_vec();
        if let Err(index) = sources.binary_search_by_key(&NODE_CONSOLE_SCOPE_SPECIFIER, |(s, _)| *s)
        {
            sources.insert(index, (NODE_CONSOLE_SCOPE_SPECIFIER, NODE_CONSOLE_SCOPE_SOURCE));
        }
        sources
    });
    LazyLock::force(&SOURCES).as_slice()
}

fn sync_bootstrap_options(runtime: &JsRuntime, has_node_modules_dir: bool) {
    let state = runtime.op_state();
    let mut state = state.borrow_mut();
    let mut options = state.try_take::<BootstrapOptions>().unwrap_or_else(|| BootstrapOptions {
        args: vec!["--colors".to_string()],
        ..BootstrapOptions::default()
    });
    options.has_node_modules_dir = has_node_modules_dir;
    state.put(options);
}

static RUNTIME_SNAPSHOT: &[u8] = include_bytes!("../../../snapshots/RARI_SNAPSHOT.bin");
include!("../../../snapshots/residual_lazy_sources.rs");

pub fn build_js_runtime(
    env_vars: Option<FxHashMap<String, String>>,
) -> Result<(JsRuntime, Rc<RariModuleLoader>), RariError> {
    let streaming_ops = get_streaming_ops();

    let ext_options = ext::ExtensionOptions::default();
    let module_loader = Rc::new(RariModuleLoader::new(Arc::clone(&ext_options.node_resolver)));
    let has_node_modules_dir = ext_options.node_resolver.has_node_modules_dir();
    let (mut extensions, lazy_args) = ext::extensions_with_lazy_args(&ext_options, true);

    extensions.push(Extension {
        name: "rari:streaming",
        ops: Cow::Owned(streaming_ops),
        op_state_fn: Some(Box::new(|state| {
            state.put(StreamOpState::default());
            state.put(ops::RequestContextStore::default());
            state.put(ops::rari_main_module());
            let feature_checker = deno_features::FeatureChecker::default();
            state.put(Arc::new(feature_checker));
        })),
        ..Default::default()
    });

    let options = RuntimeOptions {
        #[expect(
            clippy::clone_on_ref_ptr,
            reason = "Trait object coercion: Rc<RariModuleLoader> -> Rc<dyn ModuleLoader>"
        )]
        module_loader: Some(module_loader.clone()),
        extensions,
        extension_transpiler: Some(module_loader.as_extension_transpiler()),
        startup_snapshot: Some(RUNTIME_SNAPSHOT),
        residual_lazy_esm_sources: residual_lazy_esm_sources(),
        residual_lazy_js_sources: RESIDUAL_LAZY_JS_SOURCES,
        create_params: Some(runtime_create_params()),
        ..Default::default()
    };

    let mut runtime = JsRuntime::new(options);

    runtime
        .lazy_init_extensions(lazy_args)
        .map_err(|err| RariError::js_runtime(format!("Failed to lazy-init extensions: {err}")))?;

    sync_bootstrap_options(&runtime, has_node_modules_dir);

    runtime.execute_script("node_bootstrap.js", NODE_BOOTSTRAP_SCRIPT.to_string()).map_err(
        |err| RariError::js_runtime(format!("Failed to stash node bootstrap args: {err}")),
    )?;

    if let Some(env_vars) = env_vars {
        let env_script = ENV_INJECTION_SCRIPT.cow_replace(
            "const envVars = {};",
            &format!(
                "const envVars = {};",
                serde_json::to_string(&env_vars).unwrap_or_else(|_| "{}".to_string())
            ),
        );

        if let Err(err) = runtime.execute_script("env_vars.js", env_script.into_owned()) {
            eprintln!("[rari] Failed to inject environment variables: {err}");
        }
    }

    if let Err(err) =
        runtime.execute_script("module_registration_check.js", MODULE_CHECK_SCRIPT.to_string())
    {
        eprintln!("[rari] Failed to check module registration extension: {err}");
    }

    Ok((runtime, module_loader))
}

fn get_streaming_ops() -> Vec<deno_core::OpDecl> {
    ops::get_streaming_ops()
}
