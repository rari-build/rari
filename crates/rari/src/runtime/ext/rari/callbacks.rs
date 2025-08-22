use crate::error::RariError as Error;
use crate::runtime::JsError;
use deno_core::{OpState, op2, v8};
use rustc_hash::FxHashMap;
use std::{cell::RefCell, future::Future, pin::Pin, rc::Rc};

pub trait RsStoredCallback: 'static {
    #[allow(dead_code)]
    fn call(
        &self,
        args: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, Error>>>>;

    #[allow(unused)]
    fn encode_args(
        &self,
        args: v8::Global<v8::Value>,
        scope: &mut v8::HandleScope<'_>,
    ) -> Result<serde_json::Value, Error>;
}

pub trait RsCallback: 'static {
    type Arguments: serde::ser::Serialize + serde::de::DeserializeOwned;
    type Return: serde::ser::Serialize + 'static;

    async fn body(args: Self::Arguments) -> Result<Self::Return, Error>;

    #[allow(unused)]
    fn args_from_v8(
        args: Vec<v8::Global<v8::Value>>,
        scope: &mut v8::HandleScope,
    ) -> Result<Self::Arguments, Error>;

    #[allow(unused)]
    fn slow_args_from_v8(
        args: Vec<v8::Global<v8::Value>>,
        scope: &mut v8::HandleScope,
    ) -> Result<serde_json::Value, Error> {
        let args = Self::args_from_v8(args, scope)?;
        serde_json::to_value(args)
            .map_err(|e| Error::Serialization(format!("Error serializing arguments: {e}"), None))
    }

    #[allow(unused)]
    fn decode_v8(
        args: v8::Global<v8::Value>,
        scope: &mut v8::HandleScope,
    ) -> Result<Self::Arguments, Error> {
        let args = v8::Local::new(scope, args);
        let args = if args.is_array() {
            let args: v8::Local<v8::Array> = v8::Local::new(scope, args).try_into()?;
            let len = args.length() as usize;
            let mut result = Vec::with_capacity(len);
            for i in 0..len {
                let index = v8::Integer::new(
                    scope,
                    i.try_into().map_err(|_| {
                        Error::JsRuntime(
                            format!("Could not decode {len} arguments - use `big_json_args`"),
                            None,
                        )
                    })?,
                );
                let arg = args.get(scope, index.into()).ok_or_else(|| {
                    Error::JsRuntime(format!("Invalid argument at index {i}"), None)
                })?;
                result.push(v8::Global::new(scope, arg));
            }
            result
        } else {
            vec![v8::Global::new(scope, args)]
        };

        Self::args_from_v8(args, scope)
    }

    #[allow(unused)]
    async fn call(
        args: v8::Global<v8::Value>,
        scope: &mut v8::HandleScope<'_>,
    ) -> Result<Self::Return, Error> {
        let args = Self::decode_v8(args, scope)?;
        Self::body(args).await
    }
}

#[allow(unused)]
macro_rules! codegen_function {
    ($(#[doc = $doc:literal])* fn $name:ident ($($n:ident:$t:ty),+ $(,)?) -> $r:ty $body:block ) => {
        paste! {
            #[allow(non_camel_case_types)]
            $(#[doc = $doc])*
            struct [< rscallback_ $name >]();
            impl RsCallback for [< rscallback_ $name >] {
                type Arguments = ($($t,)+);
                type Return = $r;

                fn args_from_v8(
                    args: Vec<v8::Global<v8::Value>>,
                    scope: &mut v8::HandleScope,
                ) -> Result<Self::Arguments, Error> {
                    let mut args = args.into_iter();
                    $(
                        let next = args.next().ok_or(Error::JsRuntime(format!("Missing argument {} for {}", stringify!($n), stringify!($name)), None))?;
                        let next = v8::Local::new(scope, next);
                        let $n:$t = deno_core::serde_v8::from_v8(scope, next).map_err(|e| Error::Deserialization(format!("Error deserializing argument {}: {}", stringify!($n), e), None))?;
                    )+
                    Ok(($($n,)+))
                }

                async fn body(($($n,)+): Self::Arguments) -> Result<Self::Return, Error> {
                    $body
                }
            }
            impl RsStoredCallback for [< rscallback_ $name >] {
                fn call(&self, args: serde_json::Value)
                    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, Error>>>> {
                    Box::pin(async move {
                        let args: <Self as RsCallback>::Arguments = serde_json::from_value(args).map_err(|e| Error::Deserialization(format!("Error deserializing arguments: {}", e), None))?;

                        let v = Self::body(args).await?;
                        serde_json::to_value(v).map_err(|e| Error::Serialization(format!("Error serializing return value: {}", e), None))
                    })
                }

                fn encode_args(&self, args: v8::Global<v8::Value>, scope: &mut v8::HandleScope<'_>) -> Result<serde_json::Value, Error> {
                    let args = Self::decode_v8(args, scope)?;
                    serde_json::to_value(args).map_err(|e| Error::Serialization(format!("Error serializing arguments: {}", e), None))
                }
            }
        }
    }
}

#[allow(unused)]
macro_rules! rs_fn {
    ($($(#[doc = $doc:literal])* fn $name:ident ($($n:ident:$t:ty),+ $(,)?) -> $r:ty $body:block )+) => {
        $(codegen_function! { fn $name ($($n:$t),+) -> $r $body })+
    }
}

#[allow(dead_code)]
#[op2(async)]
#[serde]
pub async fn run_rscallback<T: RsCallback>(
    #[serde] args: T::Arguments,
) -> Result<T::Return, JsError> {
    T::body(args).await.map_err(JsError::from)
}

#[allow(dead_code)]
type CallbackTable = FxHashMap<String, Rc<Box<dyn RsStoredCallback>>>;

#[allow(dead_code)]
fn find_callback(name: &str, state: &OpState) -> Result<Rc<Box<dyn RsStoredCallback>>, JsError> {
    state
        .try_borrow::<CallbackTable>()
        .and_then(|t| t.get(name).cloned())
        .ok_or_else(|| JsError::generic(format!("Callback '{name}' is not callable")))
}

#[allow(dead_code)]
#[op2(async)]
#[serde]
pub async fn rscallback(
    #[string] name: String,
    #[serde] args: serde_json::Value,
    state: Rc<RefCell<OpState>>,
) -> Result<serde_json::Value, JsError> {
    let name_clone = name.clone();

    let callback = {
        let state_ref = state.borrow();
        find_callback(&name_clone, &state_ref)?
    };

    callback.call(args).await.map_err(JsError::from)
}
