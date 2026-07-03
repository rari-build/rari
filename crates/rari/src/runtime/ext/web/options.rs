#![expect(
    clippy::unnecessary_wraps,
    reason = "RequestBuilderHook type requires Result signature even when function never errors"
)]

use std::{rc::Rc, sync::Arc};

use deno_core::ModuleSpecifier;
use deno_error::JsErrorBox;
use deno_fetch::{DefaultFileFetchHandler, FetchHandler, ReqBody, dns::Resolver};
use deno_telemetry::OtelConfig;
use deno_tls::{Proxy, RootCertStoreProvider, TlsKeys};
use deno_web::{BlobStore, InMemoryBroadcastChannel};
use http::{HeaderValue, Request, header::ACCEPT_ENCODING};
use hyper_util::client::legacy::Builder;

use super::{DefaultWebPermissions, WebPermissions};

type RequestBuilderHook = fn(&mut Request<ReqBody>) -> Result<(), JsErrorBox>;

#[derive(Clone)]
pub struct WebOptions {
    pub base_url: Option<ModuleSpecifier>,
    pub user_agent: String,
    pub root_cert_store_provider: Option<Arc<dyn RootCertStoreProvider>>,
    pub proxy: Option<Proxy>,
    pub request_builder_hook: Option<RequestBuilderHook>,
    pub unsafely_ignore_certificate_errors: Option<Vec<String>>,
    pub client_cert_chain_and_key: TlsKeys,
    pub file_fetch_handler: Rc<dyn FetchHandler>,
    pub permissions: Arc<dyn WebPermissions>,
    pub blob_store: Arc<BlobStore>,
    pub broadcast_channel: InMemoryBroadcastChannel,
    pub client_builder_hook: Option<fn(Builder) -> Builder>,
    pub resolver: Resolver,
    pub telemetry_config: OtelConfig,
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
            client_cert_chain_and_key: TlsKeys::Null,
            file_fetch_handler: Rc::new(DefaultFileFetchHandler),
            permissions: Arc::new(DefaultWebPermissions),
            blob_store: Arc::new(BlobStore::default()),
            broadcast_channel: InMemoryBroadcastChannel::default(),
            client_builder_hook: None,
            resolver: Resolver::default(),
            telemetry_config: OtelConfig::default(),
        }
    }
}

/// Deno's fetch only supports gzip and deflate, not zstd or brotli.
/// This prevents issues where servers return zstd-compressed responses that Deno can't handle.
fn fix_accept_encoding_for_deno(req: &mut Request<ReqBody>) -> Result<(), JsErrorBox> {
    if !req.headers().contains_key(ACCEPT_ENCODING) {
        req.headers_mut().insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate"));
    }
    Ok(())
}

impl WebOptions {
    pub fn whitelist_certificate_for(&mut self, domain_or_ip: &impl ToString) {
        if let Some(ref mut domains) = self.unsafely_ignore_certificate_errors {
            domains.push(domain_or_ip.to_string());
        } else {
            self.unsafely_ignore_certificate_errors = Some(vec![domain_or_ip.to_string()]);
        }
    }
}
