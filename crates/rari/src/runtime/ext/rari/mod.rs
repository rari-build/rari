use super::ExtensionTrait;
use crate::runtime::{JsError, RsFunction};
use deno_core::{Extension, OpState, extension, op2, v8};
use rustc_hash::FxHashMap;

type FnCache = FxHashMap<String, Box<dyn RsFunction>>;

mod callbacks;

#[op2(fast)]
fn op_register_entrypoint(
    scope: &mut v8::PinScope<'_, '_>,
    state: &mut OpState,
    callback: v8::Local<v8::Function>,
) {
    let global_callback = v8::Global::new(scope, callback);
    state.put(global_callback);
}

#[op2]
#[serde]
fn call_registered_function(
    #[string] name: &str,
    #[serde] args: Vec<serde_json::Value>,
    state: &mut OpState,
) -> Result<serde_json::Value, JsError> {
    if state.has::<FnCache>() {
        let table = state.borrow_mut::<FnCache>();
        if let Some(callback) = table.get(name) {
            return callback.call(&args).map_err(JsError::from);
        }
    }

    Err(JsError::generic(format!("Function '{name}' is not callable")))
}

#[op2(fast)]
fn op_panic2(#[string] msg: &str) -> Result<(), JsError> {
    Err(JsError::generic(msg.to_string()))
}

extension!(
    rari,
    ops = [op_register_entrypoint, call_registered_function],
    esm_entry_point = "ext:rari/rari.js",
    esm = [ dir "src/runtime/ext/rari", "rari.js" ],
    middleware = |op| match op.name {
        "op_panic" => op.with_implementation_from(&op_panic2()),
        _ => op,
    }
);
impl ExtensionTrait<()> for rari {
    #[allow(unused_variables)]
    fn init(options: ()) -> Extension {
        rari::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![rari::build((), is_snapshot)]
}
