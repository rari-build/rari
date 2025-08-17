use std::borrow::Cow;

use super::{ExtensionTrait, web::PermissionsContainer};
use deno_core::{Extension, extension};

extension!(
    init_napi,
    deps = [rari],
    esm_entry_point = "ext:init_napi/init_napi.js",
    esm = [ dir "src/runtime/ext/napi", "init_napi.js" ],
);
impl ExtensionTrait<()> for init_napi {
    fn init((): ()) -> Extension {
        init_napi::init()
    }
}
impl ExtensionTrait<()> for deno_napi::deno_napi {
    fn init((): ()) -> Extension {
        deno_napi::deno_napi::init::<PermissionsContainer>(None)
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_napi::deno_napi::build((), is_snapshot), init_napi::build((), is_snapshot)]
}

impl deno_napi::NapiPermissions for PermissionsContainer {
    fn check<'a>(
        &mut self,
        path: Cow<'a, std::path::Path>,
    ) -> Result<Cow<'a, std::path::Path>, deno_permissions::PermissionCheckError> {
        let p = self.0.check_read(path, None)?;
        Ok(p)
    }
}
