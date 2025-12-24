use super::{DefaultWebPermissions, WebPermissions};
use deno_fetch::dns::Resolver;
use hyper_util::client::legacy::Builder;
use std::sync::Arc;

type RequestBuilderHook =
    fn(&mut http::Request<deno_fetch::ReqBody>) -> Result<(), deno_error::JsErrorBox>;

#[derive(Clone)]
pub struct WebOptions {
    pub base_url: Option<deno_core::ModuleSpecifier>,
    pub user_agent: String,
    pub root_cert_store_provider: Option<std::sync::Arc<dyn deno_tls::RootCertStoreProvider>>,
    pub proxy: Option<deno_tls::Proxy>,
    pub request_builder_hook: Option<RequestBuilderHook>,
    pub unsafely_ignore_certificate_errors: Option<Vec<String>>,
    pub client_cert_chain_and_key: deno_tls::TlsKeys,
    pub file_fetch_handler: std::rc::Rc<dyn deno_fetch::FetchHandler>,
    pub permissions: Arc<dyn WebPermissions>,
    pub blob_store: Arc<deno_web::BlobStore>,
    pub broadcast_channel: deno_web::InMemoryBroadcastChannel,
    pub client_builder_hook: Option<fn(Builder) -> Builder>,
    pub resolver: Resolver,
    pub telemetry_config: deno_telemetry::OtelConfig,
}

impl Default for WebOptions {
    fn default() -> Self {
        Self {
            base_url: None,
            user_agent: String::new(),
            root_cert_store_provider: None,
            proxy: None,
            request_builder_hook: Some(fix_accept_encoding_for_deno),
            unsafely_ignore_certificate_errors: None,
            client_cert_chain_and_key: deno_tls::TlsKeys::Null,
            file_fetch_handler: std::rc::Rc::new(deno_fetch::DefaultFileFetchHandler),
            permissions: Arc::new(DefaultWebPermissions),
            blob_store: Arc::new(deno_web::BlobStore::default()),
            broadcast_channel: deno_web::InMemoryBroadcastChannel::default(),
            client_builder_hook: None,
            resolver: Resolver::default(),
            telemetry_config: deno_telemetry::OtelConfig::default(),
        }
    }
}

/// Deno's fetch only supports gzip and deflate, not zstd or brotli.
/// This prevents issues where servers return zstd-compressed responses that Deno can't handle.
fn fix_accept_encoding_for_deno(
    req: &mut http::Request<deno_fetch::ReqBody>,
) -> Result<(), deno_error::JsErrorBox> {
    use http::header::{ACCEPT_ENCODING, HeaderValue};

    if !req.headers().contains_key(ACCEPT_ENCODING) {
        req.headers_mut().insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate"));
    }

    Ok(())
}

impl WebOptions {
    pub fn whitelist_certificate_for(&mut self, domain_or_ip: impl ToString) {
        if let Some(ref mut domains) = self.unsafely_ignore_certificate_errors {
            domains.push(domain_or_ip.to_string());
        } else {
            self.unsafely_ignore_certificate_errors = Some(vec![domain_or_ip.to_string()]);
        }
    }
}
