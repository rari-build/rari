use std::sync::OnceLock;

use reqwest::Client;

static HTTP_CLIENT: OnceLock<Result<Client, reqwest::Error>> = OnceLock::new();

#[expect(clippy::missing_errors_doc)]
pub fn get_http_client() -> Result<&'static Client, String> {
    HTTP_CLIENT
        .get_or_init(|| Client::builder().build())
        .as_ref()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}
