use crate::error::RariError;
use crate::runtime::module_loader::RariModuleLoader;
use crate::runtime::runtime_factory::constants::{ENV_INJECTION_SCRIPT, MODULE_CHECK_SCRIPT};
use deno_core::{Extension, JsRuntime, RuntimeOptions};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::rc::Rc;

#[derive(Default)]
struct StreamOpState;

pub fn create_deno_runtime(
    env_vars: Option<FxHashMap<String, String>>,
) -> Result<(JsRuntime, Rc<RariModuleLoader>), RariError> {
    let module_loader = Rc::new(RariModuleLoader::new());

    let streaming_ops = get_streaming_ops();

    let ext_options = crate::runtime::ext::ExtensionOptions::default();
    let mut extensions = crate::runtime::ext::extensions(&ext_options, false);

    extensions.push(Extension {
        name: "rari:streaming",
        ops: Cow::Owned(streaming_ops),
        op_state_fn: Some(Box::new(|state| {
            state.put(StreamOpState);
            let mut feature_checker = deno_features::FeatureChecker::default();
            feature_checker.enable_feature("broadcast-channel");
            feature_checker.enable_feature("webgpu");
            feature_checker.enable_feature("kv");
            feature_checker.enable_feature("cron");
            feature_checker.enable_feature("net");
            feature_checker.enable_feature("worker-options");
            state.put(std::sync::Arc::new(feature_checker));
        })),
        ..Default::default()
    });

    let options = RuntimeOptions {
        module_loader: Some(module_loader.clone()),
        extensions,
        extension_transpiler: Some(module_loader.as_extension_transpiler()),
        ..Default::default()
    };

    let mut runtime = JsRuntime::new(options);

    if let Some(env_vars) = env_vars {
        let env_script = ENV_INJECTION_SCRIPT.replace(
            "const envVars = {};",
            &format!(
                "const envVars = {};",
                serde_json::to_string(&env_vars).unwrap_or_else(|_| "{}".to_string())
            ),
        );

        if let Err(err) = runtime.execute_script("env_vars.js", env_script) {
            eprintln!("[RARI_WARN] Failed to inject environment variables: {err}");
        }
    }

    if let Err(err) =
        runtime.execute_script("module_registration_check.js", MODULE_CHECK_SCRIPT.to_string())
    {
        eprintln!("[RARI_WARN] Failed to check module registration extension: {err}");
    }

    Ok((runtime, module_loader))
}

fn get_streaming_ops() -> Vec<deno_core::OpDecl> {
    crate::runtime::ops::get_streaming_ops()
}
