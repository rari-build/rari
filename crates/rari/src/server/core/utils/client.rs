use std::sync::OnceLock;

use rari_error::RariError;
use reqwest::Client;

static HTTP_CLIENT: OnceLock<Result<Client, reqwest::Error>> = OnceLock::new();

#[expect(clippy::missing_errors_doc)]
pub fn get_http_client() -> Result<&'static Client, RariError> {
    HTTP_CLIENT
        .get_or_init(|| Client::builder().build())
        .as_ref()
        .map_err(|e| RariError::network(format!("Failed to create HTTP client: {e}")))
}
