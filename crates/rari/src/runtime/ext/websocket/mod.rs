use super::{ExtensionTrait, web::PermissionsContainer, web::WebOptions};
use deno_core::{Extension, extension, url::Url};
use deno_permissions::PermissionCheckError;

impl deno_websocket::WebSocketPermissions for PermissionsContainer {
    fn check_net_url(&mut self, url: &Url, api_name: &str) -> Result<(), PermissionCheckError> {
        self.0.check_url(url, api_name)?;
        Ok(())
    }
}

extension!(
    init_websocket,
    deps = [rari],
    esm_entry_point = "ext:init_websocket/init_websocket.js",
    esm = [ dir "src/runtime/ext/websocket", "init_websocket.js" ],
);
impl ExtensionTrait<()> for init_websocket {
    fn init((): ()) -> Extension {
        init_websocket::init()
    }
}
impl ExtensionTrait<WebOptions> for deno_websocket::deno_websocket {
    fn init(options: WebOptions) -> Extension {
        deno_websocket::deno_websocket::init::<PermissionsContainer>(
            options.user_agent,
            options.root_cert_store_provider,
            options.unsafely_ignore_certificate_errors,
        )
    }
}

pub fn extensions(options: WebOptions, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_websocket::deno_websocket::build(options, is_snapshot),
        init_websocket::build((), is_snapshot),
    ]
}
