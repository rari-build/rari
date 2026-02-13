use std::sync::OnceLock;

static HTTP_CLIENT: OnceLock<Result<reqwest::Client, reqwest::Error>> = OnceLock::new();

pub fn get_http_client() -> Result<&'static reqwest::Client, String> {
    HTTP_CLIENT
        .get_or_init(|| reqwest::Client::builder().build())
        .as_ref()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}
